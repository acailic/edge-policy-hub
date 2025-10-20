use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("Invalid field path: {0}")]
    InvalidPath(String),

    #[error("Maximum transformation depth exceeded")]
    MaxDepthExceeded,

    #[error("Unsupported payload format")]
    UnsupportedFormat,
}

impl From<serde_json::Error> for TransformError {
    fn from(err: serde_json::Error) -> Self {
        TransformError::InvalidJson(err.to_string())
    }
}
