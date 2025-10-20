use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Tenant ID not found in certificate or JWT")]
    TenantIdNotFound,

    #[error("Invalid certificate: {0}")]
    InvalidCertificate(String),

    #[error("Invalid JWT: {0}")]
    InvalidJwt(String),

    #[error("Missing Authorization header")]
    MissingAuthHeader,

    #[error("Unsupported authentication method")]
    UnsupportedAuthMethod,

    #[error("Tenant ID mismatch: certificate has {cert_tenant}, JWT has {jwt_tenant}")]
    TenantIdMismatch {
        cert_tenant: String,
        jwt_tenant: String,
    },
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AuthError::InvalidJwt(err.to_string())
    }
}

impl From<x509_parser::error::X509Error> for AuthError {
    fn from(err: x509_parser::error::X509Error) -> Self {
        AuthError::InvalidCertificate(err.to_string())
    }
}

impl From<x509_parser::nom::Err<x509_parser::error::X509Error>> for AuthError {
    fn from(err: x509_parser::nom::Err<x509_parser::error::X509Error>) -> Self {
        match err {
            x509_parser::nom::Err::Error(e) | x509_parser::nom::Err::Failure(e) => {
                AuthError::InvalidCertificate(e.to_string())
            }
            x509_parser::nom::Err::Incomplete(_) => {
                AuthError::InvalidCertificate("Incomplete certificate".to_string())
            }
        }
    }
}
