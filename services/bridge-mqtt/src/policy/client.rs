use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use super::{MqttAbacInput, PolicyError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryRequest {
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryResponse {
    pub result: PolicyDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    #[serde(default)]
    pub redact: Option<Vec<String>>,
    #[serde(default)]
    pub redact_fields: Option<Vec<String>>,
    #[serde(default)]
    pub remove_fields: Option<Vec<String>>,
    #[serde(default)]
    pub strip_coordinates: Option<bool>,
    #[serde(default)]
    pub reason: Option<String>,
}

pub struct PolicyClient {
    http_client: reqwest::Client,
    enforcer_base_url: String,
    timeout: Duration,
    use_mqtt_endpoints: bool,
}

impl PolicyClient {
    pub fn new(enforcer_url: String, timeout_secs: u64, use_mqtt_endpoints: bool) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(10)
            .build()?;

        // Validate URL format
        if !enforcer_url.starts_with("http://") && !enforcer_url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "Invalid enforcer URL: must start with http:// or https://"
            ));
        }

        Ok(Self {
            http_client,
            enforcer_base_url: enforcer_url.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(timeout_secs),
            use_mqtt_endpoints,
        })
    }

    pub async fn query_publish_policy(
        &self,
        tenant_id: &str,
        input: MqttAbacInput,
    ) -> Result<PolicyDecision, PolicyError> {
        let start = std::time::Instant::now();

        let decision = if self.use_mqtt_endpoints {
            // Try MQTT-specific publish endpoint first
            let mqtt_url = format!(
                "{}/v1/data/tenants/{}/mqtt/publish",
                self.enforcer_base_url, tenant_id
            );

            let result = self.query_policy(&mqtt_url, &input).await;

            // Fallback to generic allow endpoint if MQTT-specific not found
            match result {
                Err(PolicyError::TenantNotFound(_)) | Err(PolicyError::EnforcerError { status: 404, .. }) => {
                    debug!("MQTT-specific publish policy not found, falling back to generic allow endpoint");
                    let generic_url = format!(
                        "{}/v1/data/tenants/{}/allow",
                        self.enforcer_base_url, tenant_id
                    );
                    self.query_policy(&generic_url, &input).await?
                }
                other => other?,
            }
        } else {
            // Directly use generic allow endpoint
            let generic_url = format!(
                "{}/v1/data/tenants/{}/allow",
                self.enforcer_base_url, tenant_id
            );
            self.query_policy(&generic_url, &input).await?
        };

        let latency = start.elapsed();
        debug!(
            "Publish policy query: tenant={}, topic={}, allow={}, latency={:?}",
            tenant_id, input.resource.topic, decision.allow, latency
        );

        Ok(decision)
    }

    pub async fn query_subscribe_policy(
        &self,
        tenant_id: &str,
        input: MqttAbacInput,
    ) -> Result<PolicyDecision, PolicyError> {
        let start = std::time::Instant::now();

        let decision = if self.use_mqtt_endpoints {
            // Try MQTT-specific subscribe endpoint first
            let mqtt_url = format!(
                "{}/v1/data/tenants/{}/mqtt/subscribe",
                self.enforcer_base_url, tenant_id
            );

            let result = self.query_policy(&mqtt_url, &input).await;

            // Fallback to generic allow endpoint if MQTT-specific not found
            match result {
                Err(PolicyError::TenantNotFound(_)) | Err(PolicyError::EnforcerError { status: 404, .. }) => {
                    debug!("MQTT-specific subscribe policy not found, falling back to generic allow endpoint");
                    let generic_url = format!(
                        "{}/v1/data/tenants/{}/allow",
                        self.enforcer_base_url, tenant_id
                    );
                    self.query_policy(&generic_url, &input).await?
                }
                other => other?,
            }
        } else {
            // Directly use generic allow endpoint
            let generic_url = format!(
                "{}/v1/data/tenants/{}/allow",
                self.enforcer_base_url, tenant_id
            );
            self.query_policy(&generic_url, &input).await?
        };

        let latency = start.elapsed();
        debug!(
            "Subscribe policy query: tenant={}, topic_filter={}, allow={}, latency={:?}",
            tenant_id, input.resource.topic, decision.allow, latency
        );

        Ok(decision)
    }

    async fn query_policy(
        &self,
        url: &str,
        input: &MqttAbacInput,
    ) -> Result<PolicyDecision, PolicyError> {
        let request = PolicyQueryRequest {
            input: serde_json::to_value(input).map_err(|e| {
                PolicyError::InvalidResponse(format!("Failed to serialize input: {}", e))
            })?,
        };

        let response = self
            .http_client
            .post(url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let policy_response: PolicyQueryResponse = response.json().await.map_err(|e| {
                PolicyError::InvalidResponse(format!("Failed to parse response: {}", e))
            })?;

            if !policy_response.result.allow {
                return Err(PolicyError::Denied {
                    reason: policy_response.result.reason,
                });
            }

            Ok(policy_response.result)
        } else if status.as_u16() == 404 {
            Err(PolicyError::TenantNotFound(
                "Policy endpoint not found".to_string(),
            ))
        } else if status.as_u16() == 403 {
            Err(PolicyError::Denied {
                reason: Some("Access denied by policy".to_string()),
            })
        } else {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(PolicyError::EnforcerError {
                status: status.as_u16(),
                message: error_body,
            })
        }
    }
}
