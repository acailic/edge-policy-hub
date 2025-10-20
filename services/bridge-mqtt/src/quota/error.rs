use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuotaError {
    #[error("Quota limit exceeded for tenant '{tenant_id}': {current} / {limit}")]
    LimitExceeded {
        tenant_id: String,
        limit: u64,
        current: u64,
    },

    #[error("Invalid tenant ID: {0}")]
    InvalidTenantId(String),
}
