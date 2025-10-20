use serde::{Deserialize, Serialize};

use crate::tracker::QuotaMetrics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementQuotaRequest {
    pub tenant_id: String,
    pub message_count: Option<u64>,
    pub bytes_sent: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementQuotaResponse {
    pub metrics: QuotaMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetQuotaResponse {
    pub metrics: QuotaMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLimitsRequest {
    pub tenant_id: String,
    pub message_limit: u64,
    pub bandwidth_limit_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLimitsResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckQuotaRequest {
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckQuotaResponse {
    pub exceeded: bool,
    pub quota_type: Option<String>,
    pub limit: Option<u64>,
    pub current: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
}
