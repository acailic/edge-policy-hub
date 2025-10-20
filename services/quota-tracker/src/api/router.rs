use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use super::handlers;
use super::ApiState;

pub fn create_router(state: Arc<ApiState>) -> Router {
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .timeout(Duration::from_secs(30));

    Router::new()
        .route("/api/quota/increment", post(handlers::increment_quota))
        .route("/api/quota/check", post(handlers::check_quota))
        .route("/api/quota/limits", post(handlers::set_limits))
        .route("/api/quota", get(handlers::list_quotas))
        .route("/api/quota/:tenant_id", get(handlers::get_quota))
        .route("/api/quota/:tenant_id/reset", post(handlers::reset_quota))
        .route("/health", get(handlers::health_check))
        .with_state(state)
        .layer(middleware)
}
