use thiserror::Error;

use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum QuotaError {
    #[error("quota exceeded for tenant {tenant_id} ({quota_type}): limit={limit}, current={current}")]
    LimitExceeded {
        tenant_id: String,
        quota_type: String,
        limit: u64,
        current: u64,
    },
    #[error("tenant {0} not found")]
    TenantNotFound(String),
    #[error("invalid period: {0}")]
    InvalidPeriod(String),
    #[error("storage error: {0}")]
    StorageError(#[from] StorageError),
}
