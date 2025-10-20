use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Tenant ID not found in certificate, username, or client ID")]
    TenantIdNotFound,

    #[error("Invalid certificate: {0}")]
    InvalidCertificate(String),

    #[error("Invalid username format: {0}")]
    InvalidUsernameFormat(String),

    #[error("Invalid client ID format: {0}")]
    InvalidClientIdFormat(String),

    #[error("Tenant ID mismatch: certificate tenant '{cert_tenant}' does not match username tenant '{username_tenant}'")]
    TenantIdMismatch {
        cert_tenant: String,
        username_tenant: String,
    },

    #[error("Tenant ID is empty")]
    EmptyTenantId,
}

impl From<x509_parser::error::X509Error> for AuthError {
    fn from(err: x509_parser::error::X509Error) -> Self {
        AuthError::InvalidCertificate(err.to_string())
    }
}

impl<E> From<x509_parser::nom::Err<E>> for AuthError
where
    E: std::fmt::Display + std::fmt::Debug,
{
    fn from(err: x509_parser::nom::Err<E>) -> Self {
        AuthError::InvalidCertificate(format!("Certificate parsing error: {}", err))
    }
}
