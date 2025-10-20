use serde::ser::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("{service} request failed with status {status}: {message}")]
    ApiError {
        service: String,
        status: u16,
        message: String,
    },
    #[error("network error: {0}")]
    NetworkError(String),
    #[error("validation error: {0}")]
    ValidationError(String),
    #[error("failed to parse response: {0}")]
    SerializationError(String),
    #[error("resource not found: {0}")]
    NotFound(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<reqwest::Error> for CommandError {
    fn from(error: reqwest::Error) -> Self {
        if let Some(status) = error.status() {
            return CommandError::ApiError {
                service: "remote service".to_string(),
                status: status.as_u16(),
                message: error.to_string(),
            };
        }

        if error.is_timeout() {
            return CommandError::NetworkError("request timed out".to_string());
        }

        if error.is_connect() {
            return CommandError::NetworkError("failed to connect to service".to_string());
        }

        CommandError::NetworkError(error.to_string())
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(error: serde_json::Error) -> Self {
        CommandError::SerializationError(error.to_string())
    }
}
