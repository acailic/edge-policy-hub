use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::storage::tenant_registry::TenantRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRequest {
    pub tenant_id: String,
    pub timestamp: String,
    pub decision: String,
    pub protocol: String,
    pub subject: Value,
    pub action: String,
    pub resource: Value,
    pub environment: Value,
    pub policy_version: Option<u32>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub log_id: String,
    pub signature: String,
    pub stored_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryLogsRequest {
    pub tenant_id: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub decision: Option<String>,
    pub protocol: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryLogsResponse {
    pub logs: Vec<AuditLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub log_id: String,
    pub tenant_id: String,
    pub timestamp: String,
    pub decision: String,
    pub protocol: String,
    pub subject: Value,
    pub action: String,
    pub resource: Value,
    pub environment: Value,
    pub policy_version: Option<u32>,
    pub reason: Option<String>,
    pub signature: String,
    pub uploaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRequest {
    pub tenant_id: String,
    pub name: String,
    pub config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantResponse {
    pub tenant_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub config: Option<Value>,
}

impl From<TenantRecord> for TenantResponse {
    fn from(record: TenantRecord) -> Self {
        Self {
            tenant_id: record.tenant_id,
            name: record.name,
            status: record.status,
            created_at: record.created_at,
            updated_at: record.updated_at,
            config: record.config,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkUploadedRequest {
    pub tenant_id: String,
    pub log_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnuploadedQuery {
    pub tenant_id: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<Value>,
}
