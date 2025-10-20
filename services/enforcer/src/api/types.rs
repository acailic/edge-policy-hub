use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryRequest {
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyQueryResponse {
    pub result: PolicyDecision,
    pub metrics: Option<EvaluationMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    #[serde(default)]
    pub redact: Option<Vec<String>>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub eval_duration_micros: u64,
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    pub event_id: String,
    pub tenant_id: String,
    pub timestamp: String,
    pub decision: PolicyDecision,
    pub input: Value,
    pub metrics: EvaluationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFilter {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
}
