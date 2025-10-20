use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{delete, get, post, put},
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
        .route("/api/audit/logs", post(handlers::write_audit_log).get(handlers::query_audit_logs))
        .route(
            "/api/audit/logs/unuploaded",
            get(handlers::get_unuploaded_logs),
        )
        .route(
            "/api/audit/logs/mark-uploaded",
            post(handlers::mark_uploaded),
        )
        .route("/api/tenants", post(handlers::create_tenant).get(handlers::list_tenants))
        .route(
            "/api/tenants/:tenant_id",
            get(handlers::get_tenant).put(handlers::update_tenant).delete(handlers::delete_tenant),
        )
        .route(
            "/api/bundles",
            post(handlers::create_policy_bundle).get(handlers::list_policy_bundles),
        )
        .route(
            "/api/bundles/:bundle_id",
            get(handlers::get_policy_bundle),
        )
        .route(
            "/api/bundles/:bundle_id/activate",
            post(handlers::activate_policy_bundle),
        )
        .route(
            "/api/bundles/:bundle_id/archive",
            post(handlers::archive_policy_bundle),
        )
        .route("/health", get(handlers::health_check))
        .with_state(state)
        .layer(middleware)
}
