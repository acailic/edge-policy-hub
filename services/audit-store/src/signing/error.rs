use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("invalid signing key: {0}")]
    InvalidKey(String),
    #[error("signature mismatch")]
    SignatureMismatch,
    #[error("encoding error: {0}")]
    EncodingError(String),
}
