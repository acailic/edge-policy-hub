use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Enforcer service unreachable: {0}")]
    EnforcerUnreachable(String),

    #[error("Enforcer returned error (status {status}): {message}")]
    EnforcerError { status: u16, message: String },

    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Policy evaluation timeout")]
    EvaluationTimeout,

    #[error("Invalid response from enforcer: {0}")]
    InvalidResponse(String),

    #[error("Policy denied: {}", .reason.as_ref().unwrap_or(&"no reason provided".to_string()))]
    Denied { reason: Option<String> },
}

impl From<reqwest::Error> for PolicyError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            PolicyError::EvaluationTimeout
        } else if err.is_connect() {
            PolicyError::EnforcerUnreachable(err.to_string())
        } else {
            PolicyError::EnforcerUnreachable(err.to_string())
        }
    }
}
