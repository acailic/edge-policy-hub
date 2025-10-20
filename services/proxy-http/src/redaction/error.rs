use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedactionError {
    #[error("Invalid redaction path: {0}")]
    InvalidPath(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("Maximum redaction depth exceeded")]
    MaxDepthExceeded,
}

impl From<serde_json::Error> for RedactionError {
    fn from(err: serde_json::Error) -> Self {
        RedactionError::JsonParseError(err.to_string())
    }
}
