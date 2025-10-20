use reqwest::Error as ReqwestError;
use thiserror::Error;

use crate::storage::error::StorageError;

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("network error: {0}")]
    NetworkError(String),
    #[error("authentication failed")]
    AuthenticationError,
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("storage error: {0}")]
    DatabaseError(#[from] StorageError),
}

impl From<ReqwestError> for UploadError {
    fn from(error: ReqwestError) -> Self {
        if error.status()
            .map(|status| status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN)
            .unwrap_or(false)
        {
            UploadError::AuthenticationError
        } else {
            UploadError::NetworkError(error.to_string())
        }
    }
}
