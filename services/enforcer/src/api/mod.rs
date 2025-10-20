use std::sync::Arc;

use axum::{
    body::Body,
    http::HeaderName,
    middleware::{self, Next},
    routing::{get, post},
    Router,
};
use tokio::sync::broadcast;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::policy::PolicyManager;

mod handlers;
mod types;
mod websocket;

pub use handlers::{health_check, query_policy, reload_tenant};
pub use types::{
    DecisionEvent, ErrorResponse, EvaluationMetrics, PolicyDecision, PolicyQueryRequest,
    PolicyQueryResponse, StreamFilter,
};
pub use websocket::ws_decision_stream;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub fn create_router(
    policy_manager: Arc<PolicyManager>,
    event_tx: Arc<broadcast::Sender<DecisionEvent>>,
) -> Router {
    Router::new()
        .route("/v1/data/tenants/:tenant_id/allow", post(query_policy))
        .route("/health", get(health_check))
        .route("/v1/tenants/:tenant_id/reload", post(reload_tenant))
        .route("/v1/stream/decisions", get(ws_decision_stream))
        .with_state((policy_manager, event_tx))
        .layer(middleware::from_fn(set_request_id))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

async fn set_request_id(
    mut request: axum::http::Request<Body>,
    next: Next,
) -> axum::response::Response {
    let request_id = Uuid::new_v4().to_string();
    request.extensions_mut().insert(request_id.clone());

    if let Ok(header_value) = axum::http::HeaderValue::from_str(&request_id) {
        request
            .headers_mut()
            .insert(REQUEST_ID_HEADER.clone(), header_value);
    }

    let mut response = next.run(request).await;

    if !response.headers().contains_key(&REQUEST_ID_HEADER) {
        if let Ok(header_value) = axum::http::HeaderValue::from_str(&request_id) {
            response
                .headers_mut()
                .insert(REQUEST_ID_HEADER.clone(), header_value);
        }
    }

    response
}
