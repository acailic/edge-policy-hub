use thiserror::Error;

mod validator;

pub use validator::{validate_tenant_id_format, validate_tenant_match};

#[derive(Debug, Error)]
pub enum TenantValidationError {
    #[error("tenant id mismatch: url='{url_tenant}', input='{input_tenant}'")]
    Mismatch {
        url_tenant: String,
        input_tenant: String,
    },
    #[error("input.subject.tenant_id is missing")]
    MissingInputTenant,
    #[error("invalid tenant id '{0}'")]
    InvalidTenantId(String),
}
