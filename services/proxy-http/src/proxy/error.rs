use crate::auth::AuthError;
use crate::policy::PolicyError;
use crate::redaction::RedactionError;
use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::Full;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Authentication failed: {0}")]
    Auth(#[from] AuthError),

    #[error("Policy enforcement failed: {0}")]
    Policy(#[from] PolicyError),

    #[error("Response redaction failed: {0}")]
    Redaction(#[from] RedactionError),

    #[error("Upstream request failed: {0}")]
    Upstream(String),

    #[error("Body too large: {size} bytes exceeds limit of {limit} bytes")]
    BodyTooLarge { size: usize, limit: usize },

    #[error("Request timeout")]
    Timeout,
}

impl ProxyError {
    pub fn to_response(&self, request_id: Option<&str>) -> Response<Full<Bytes>> {
        let (status, error_code, message) = match self {
            ProxyError::Auth(e) => (StatusCode::UNAUTHORIZED, "AUTH_ERROR", e.to_string()),
            ProxyError::Policy(e) => (
                e.to_status_code(),
                match e {
                    PolicyError::Denied { .. } => "POLICY_DENIED",
                    PolicyError::TenantNotFound(_) => "TENANT_NOT_FOUND",
                    PolicyError::EnforcerUnreachable(_) => "ENFORCER_UNREACHABLE",
                    PolicyError::EvaluationTimeout => "EVALUATION_TIMEOUT",
                    _ => "POLICY_ERROR",
                },
                e.to_string(),
            ),
            ProxyError::Redaction(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "REDACTION_ERROR",
                e.to_string(),
            ),
            ProxyError::Upstream(e) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR", e.to_string()),
            ProxyError::BodyTooLarge { size, limit } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "BODY_TOO_LARGE",
                format!("Request body size {} exceeds limit {}", size, limit),
            ),
            ProxyError::Timeout => (
                StatusCode::GATEWAY_TIMEOUT,
                "TIMEOUT",
                "Request timeout".to_string(),
            ),
        };

        let body_json = json!({
            "error": error_code,
            "message": message,
            "request_id": request_id,
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                serde_json::to_string(&body_json).unwrap(),
            )))
            .unwrap()
    }
}
