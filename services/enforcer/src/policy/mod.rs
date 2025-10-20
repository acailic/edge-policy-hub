use anyhow::Error as AnyhowError;
use thiserror::Error;

mod engine;
mod loader;
mod manager;

pub use engine::TenantEngine;
pub use loader::{BundleLoader, BundleMetadata, PolicyBundle};
pub use manager::PolicyManager;

pub type TenantId = String;

pub const DEFAULT_ENTRYPOINT_TEMPLATE: &str = "data.tenants.{tenant_id}.allow";
pub const MAX_EVAL_TIME_MS: u64 = 10;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("tenant '{0}' not found")]
    TenantNotFound(String),
    #[error("policy evaluation failed for tenant '{tenant_id}'")]
    EvaluationFailed {
        tenant_id: String,
        #[source]
        source: AnyhowError,
    },
    #[error("failed to load bundle for tenant '{tenant_id}'")]
    BundleLoadError {
        tenant_id: String,
        #[source]
        source: AnyhowError,
    },
    #[error("invalid policy for tenant '{tenant_id}': {reason}")]
    InvalidPolicy { tenant_id: String, reason: String },
}

impl PolicyError {
    pub fn tenant_id(&self) -> Option<&str> {
        match self {
            PolicyError::TenantNotFound(tenant_id) => Some(tenant_id.as_str()),
            PolicyError::EvaluationFailed { tenant_id, .. } => Some(tenant_id.as_str()),
            PolicyError::BundleLoadError { tenant_id, .. } => Some(tenant_id.as_str()),
            PolicyError::InvalidPolicy { tenant_id, .. } => Some(tenant_id.as_str()),
        }
    }
}
