use std::time::Duration;

use anyhow::{anyhow, Context};
use regorus::{Engine as RegoEngine, Value as RegoValue};
use serde_json::{json, Value as JsonValue};
use tokio::{task::spawn_blocking, time::timeout};
use tracing::{debug, instrument};

use crate::{
    api::PolicyDecision,
    policy::{PolicyError, DEFAULT_ENTRYPOINT_TEMPLATE, MAX_EVAL_TIME_MS},
};

#[derive(Clone)]
pub struct TenantEngine {
    engine: RegoEngine,
    tenant_id: String,
    entrypoint: String,
}

impl TenantEngine {
    pub fn new(
        tenant_id: String,
        policies: Vec<(String, String)>,
        data: Option<JsonValue>,
    ) -> Result<Self, PolicyError> {
        let mut engine = RegoEngine::default();

        for (filename, content) in policies {
            let policy_name = filename.clone();
            engine
                .add_policy(filename, content)
                .map_err(|err| PolicyError::InvalidPolicy {
                    tenant_id: tenant_id.clone(),
                    reason: format!("failed to load policy '{}': {}", policy_name, err),
                })?;
        }

        if let Some(data_value) = data {
            let namespaced_data = json!({
                "tenants": {
                    (tenant_id.clone()): data_value
                }
            });

            let namespaced_json = serde_json::to_string(&namespaced_data).map_err(|err| {
                PolicyError::InvalidPolicy {
                    tenant_id: tenant_id.clone(),
                    reason: format!("failed to serialize data.json: {}", err),
                }
            })?;

            engine
                .add_data_json(&namespaced_json)
                .map_err(|err| PolicyError::InvalidPolicy {
                    tenant_id: tenant_id.clone(),
                    reason: format!("failed to load data.json: {}", err),
                })?;
        }

        let entrypoint = DEFAULT_ENTRYPOINT_TEMPLATE.replace("{tenant_id}", &tenant_id);

        Ok(Self {
            engine,
            tenant_id,
            entrypoint,
        })
    }

    #[instrument(skip(self, input), fields(tenant_id = %self.tenant_id))]
    pub async fn evaluate(&self, input: JsonValue) -> Result<PolicyDecision, PolicyError> {
        let mut engine = self.engine.clone();

        let input_json = serde_json::to_string(&input)
            .context("failed to serialize evaluation input")
            .map_err(|err| PolicyError::EvaluationFailed {
                tenant_id: self.tenant_id.clone(),
                source: err.into(),
            })?;

        engine
            .set_input_json(&input_json)
            .map_err(|err| PolicyError::EvaluationFailed {
                tenant_id: self.tenant_id.clone(),
                source: err.into(),
            })?;

        let tenant_id = self.tenant_id.clone();
        let entrypoint = self.entrypoint.clone();

        let result = match timeout(
            Duration::from_millis(MAX_EVAL_TIME_MS),
            spawn_blocking(move || {
                let mut engine = engine;
                engine.eval_rule(entrypoint)
            }),
        )
        .await
        {
            Ok(Ok(Ok(value))) => value,
            Ok(Ok(Err(err))) => {
                return Err(PolicyError::EvaluationFailed {
                    tenant_id,
                    source: err.into(),
                })
            }
            Ok(Err(join_err)) => {
                return Err(PolicyError::EvaluationFailed {
                    tenant_id,
                    source: join_err.into(),
                })
            }
            Err(_) => {
                return Err(PolicyError::EvaluationFailed {
                    tenant_id,
                    source: anyhow!("policy evaluation timed out after {} ms", MAX_EVAL_TIME_MS)
                        .into(),
                })
            }
        };

        let decision = parse_decision(result);
        debug!(
            tenant = %self.tenant_id,
            allow = decision.allow,
            reason = decision.reason.as_deref().unwrap_or_default(),
            "policy evaluation completed"
        );

        Ok(decision)
    }

    pub fn verify_entrypoint(&self) -> Result<(), PolicyError> {
        let mut engine = self.engine.clone();
        let tenant_id = self.tenant_id.clone();

        engine
            .set_input_json("{}")
            .map_err(|err| PolicyError::InvalidPolicy {
                tenant_id: tenant_id.clone(),
                reason: err.to_string(),
            })?;

        match engine.eval_rule(self.entrypoint.clone()) {
            Ok(result) => {
                if matches!(result, RegoValue::Undefined) {
                    Err(PolicyError::InvalidPolicy {
                        tenant_id,
                        reason: "missing entrypoint".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
            Err(err) => Err(PolicyError::InvalidPolicy {
                tenant_id,
                reason: format!("missing entrypoint: {}", err),
            }),
        }
    }
}

fn parse_decision(result: RegoValue) -> PolicyDecision {
    match serde_json::to_value(&result) {
        Ok(JsonValue::Bool(allow)) => PolicyDecision {
            allow,
            redact: None,
            reason: None,
        },
        Ok(JsonValue::Object(map)) => {
            let allow = map.get("allow").and_then(|v| v.as_bool()).unwrap_or(false);

            let redact = map
                .get("redact")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Vec<String>>()
                })
                .filter(|items| !items.is_empty());

            let reason = map
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            PolicyDecision {
                allow,
                redact,
                reason,
            }
        }
        _ => PolicyDecision {
            allow: false,
            redact: None,
            reason: Some("policy returned undefined result".to_string()),
        },
    }
}
