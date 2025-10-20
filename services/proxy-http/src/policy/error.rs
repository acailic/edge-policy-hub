use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Enforcer service unreachable: {0}")]
    EnforcerUnreachable(String),

    #[error("Enforcer returned error (status {status}): {message}")]
    EnforcerError { status: StatusCode, message: String },

    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Policy evaluation timeout")]
    EvaluationTimeout,

    #[error("Invalid response from enforcer: {0}")]
    InvalidResponse(String),

    #[error("Request denied by policy{}", .reason.as_ref().map(|r| format!(": {}", r)).unwrap_or_default())]
    Denied { reason: Option<String> },
}

impl From<reqwest::Error> for PolicyError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            PolicyError::EvaluationTimeout
        } else {
            PolicyError::EnforcerUnreachable(err.to_string())
        }
    }
}

impl PolicyError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            PolicyError::Denied { .. } => StatusCode::FORBIDDEN,
            PolicyError::TenantNotFound(_) => StatusCode::NOT_FOUND,
            PolicyError::EnforcerUnreachable(_) | PolicyError::EvaluationTimeout => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            PolicyError::EnforcerError { .. } | PolicyError::InvalidResponse(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
