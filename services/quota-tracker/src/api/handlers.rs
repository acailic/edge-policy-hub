use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use tracing::{error, info};

use crate::tracker::QuotaError;

use super::types::{
    CheckQuotaRequest, CheckQuotaResponse, ErrorResponse, IncrementQuotaRequest,
    IncrementQuotaResponse, SetLimitsRequest, SetLimitsResponse,
};
use super::ApiState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub async fn increment_quota(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<IncrementQuotaRequest>,
) -> ApiResult<IncrementQuotaResponse> {
    if request.tenant_id.trim().is_empty() {
        return Err(bad_request("invalid_tenant_id", "tenant_id cannot be empty"));
    }

    let messages = request.message_count.unwrap_or(1);
    let bytes = request.bytes_sent.unwrap_or(0);
    let metrics = state
        .quota_manager
        .increment_message_count(&request.tenant_id, messages, bytes);

    Ok(Json(IncrementQuotaResponse { metrics }))
}

pub async fn get_quota(
    State(state): State<Arc<ApiState>>,
    Path(tenant_id): Path<String>,
) -> ApiResult<super::types::GetQuotaResponse> {
    match state.quota_manager.get_metrics(&tenant_id) {
        Some(metrics) => Ok(Json(super::types::GetQuotaResponse { metrics })),
        None => Err(not_found("tenant_not_found", "tenant not tracked")),
    }
}

pub async fn check_quota(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CheckQuotaRequest>,
) -> ApiResult<CheckQuotaResponse> {
    match state.quota_manager.check_quota(&request.tenant_id) {
        Ok(_) => Ok(Json(CheckQuotaResponse {
            exceeded: false,
            quota_type: None,
            limit: None,
            current: None,
        })),
        Err(QuotaError::LimitExceeded {
            tenant_id: _,
            quota_type,
            limit,
            current,
        }) => Ok(Json(CheckQuotaResponse {
            exceeded: true,
            quota_type: Some(quota_type),
            limit: Some(limit),
            current: Some(current),
        })),
        Err(QuotaError::TenantNotFound(_)) => Err(not_found("tenant_not_found", "tenant not tracked")),
        Err(err) => Err(internal_error(err)),
    }
}

pub async fn set_limits(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<SetLimitsRequest>,
) -> ApiResult<SetLimitsResponse> {
    if request.message_limit == 0 {
        return Err(bad_request("invalid_limit", "message_limit must be greater than zero"));
    }
    if request.bandwidth_limit_gb <= 0.0 {
        return Err(bad_request(
            "invalid_limit",
            "bandwidth_limit_gb must be greater than zero",
        ));
    }

    state
        .quota_manager
        .set_limits(&request.tenant_id, request.message_limit, request.bandwidth_limit_gb)
        .map_err(|err| internal_error(err))?;

    info!(
        tenant_id = %request.tenant_id,
        message_limit = request.message_limit,
        bandwidth_limit_gb = request.bandwidth_limit_gb,
        "quota limits updated"
    );

    Ok(Json(SetLimitsResponse { success: true }))
}

pub async fn list_quotas(
    State(state): State<Arc<ApiState>>,
) -> ApiResult<Vec<crate::tracker::QuotaMetrics>> {
    let metrics = state.quota_manager.all_metrics();
    Ok(Json(metrics))
}

pub async fn reset_quota(
    State(state): State<Arc<ApiState>>,
    Path(tenant_id): Path<String>,
) -> ApiResult<SetLimitsResponse> {
    state
        .quota_manager
        .reset_quota(&tenant_id)
        .map_err(|err| internal_error(err))?;

    Ok(Json(SetLimitsResponse { success: true }))
}

pub async fn health_check() -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "quota-tracker"
    })))
}

fn bad_request(code: &str, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
            code: code.to_string(),
            details: None,
        }),
    )
}

fn not_found(code: &str, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_string(),
            code: code.to_string(),
            details: None,
        }),
    )
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<ErrorResponse>) {
    error!(error = %err, "quota API internal error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal server error".to_string(),
            code: "internal_error".to_string(),
            details: Some(serde_json::json!({ "message": err.to_string() })),
        }),
    )
}
