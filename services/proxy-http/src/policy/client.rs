use super::{AbacInput, PolicyError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryRequest {
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryResponse {
    pub result: PolicyDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redact: Option<Vec<String>>,
}

pub struct PolicyClient {
    http_client: Client,
    enforcer_base_url: String,
}

impl PolicyClient {
    pub fn new(enforcer_url: String, timeout_secs: u64) -> anyhow::Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(10)
            .build()?;

        Ok(Self {
            http_client,
            enforcer_base_url: enforcer_url.trim_end_matches('/').to_string(),
        })
    }

    #[instrument(skip(self, input), fields(tenant_id = %tenant_id))]
    pub async fn query_policy(
        &self,
        tenant_id: &str,
        input: AbacInput,
    ) -> Result<PolicyDecision, PolicyError> {
        let url = format!(
            "{}/v1/data/tenants/{}/allow",
            self.enforcer_base_url, tenant_id
        );

        debug!("Querying policy enforcer at {}", url);

        let input_value = serde_json::to_value(&input).map_err(|e| {
            PolicyError::InvalidResponse(format!("Failed to serialize ABAC input: {}", e))
        })?;

        let request = PolicyQueryRequest {
            input: input_value,
        };

        let start = std::time::Instant::now();

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(PolicyError::from)?;

        let latency = start.elapsed();
        let status = response.status();

        if status.is_success() {
            let policy_response: PolicyQueryResponse = response.json().await.map_err(|e| {
                PolicyError::InvalidResponse(format!("Failed to parse policy response: {}", e))
            })?;

            let decision = policy_response.result;

            info!(
                tenant_id = %tenant_id,
                allow = decision.allow,
                latency_ms = latency.as_millis(),
                "Policy query completed"
            );

            if !decision.allow {
                return Err(PolicyError::Denied {
                    reason: decision.reason.clone(),
                });
            }

            Ok(decision)
        } else if status == reqwest::StatusCode::FORBIDDEN {
            // 403 Forbidden maps to policy denial
            let error_message = response
                .text()
                .await
                .unwrap_or_else(|_| "Access forbidden by policy".to_string());

            Err(PolicyError::Denied {
                reason: Some(error_message),
            })
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(PolicyError::TenantNotFound(tenant_id.to_string()))
        } else {
            let error_message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(PolicyError::EnforcerError {
                status: http::StatusCode::from_u16(status.as_u16()).unwrap(),
                message: error_message,
            })
        }
    }
}
