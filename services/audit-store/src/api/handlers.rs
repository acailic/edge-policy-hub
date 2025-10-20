use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

use crate::storage::database::LogFilter;
use crate::storage::policy_bundles::PolicyBundleRecord;
use crate::storage::tenant_registry::TenantRecord;

use super::types::{
    AuditLogEntry, AuditLogRequest, AuditLogResponse, ErrorResponse, MarkUploadedRequest,
    QueryLogsRequest, QueryLogsResponse, TenantRequest, TenantResponse, UnuploadedQuery,
    UpdateTenantRequest,
};
use super::ApiState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub async fn write_audit_log(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<AuditLogRequest>,
) -> ApiResult<AuditLogResponse> {
    if state
        .tenant_registry
        .get_tenant(&request.tenant_id)
        .map_err(|err| internal_error(err))?
        .is_none()
    {
        return Err(not_found("tenant_not_found", "tenant not registered"));
    }

    // Validate timestamp format
    if let Err(e) = DateTime::parse_from_rfc3339(&request.timestamp) {
        return Err(bad_request(
            "invalid_timestamp",
            &format!("timestamp must be valid RFC3339 format: {}", e),
        ));
    }

    let log_id = Uuid::new_v4().to_string();
    let stored_at = Utc::now().to_rfc3339();

    let mut entry = AuditLogEntry {
        log_id: log_id.clone(),
        tenant_id: request.tenant_id.clone(),
        timestamp: request.timestamp.clone(),
        decision: request.decision.clone(),
        protocol: request.protocol.clone(),
        subject: request.subject.clone(),
        action: request.action.clone(),
        resource: request.resource.clone(),
        environment: request.environment.clone(),
        policy_version: request.policy_version,
        reason: request.reason.clone(),
        signature: String::new(),
        uploaded: false,
    };

    entry.signature = state
        .signer
        .sign_audit_log(&entry)
        .map_err(|err| internal_error(err))?;

    state
        .database
        .write_audit_log(&request.tenant_id, &entry)
        .map_err(internal_error)?;

    info!(
        tenant_id = %request.tenant_id,
        log_id = %log_id,
        decision = %request.decision,
        "stored audit log"
    );

    Ok(Json(AuditLogResponse {
        log_id,
        signature: entry.signature,
        stored_at,
    }))
}

pub async fn query_audit_logs(
    State(state): State<Arc<ApiState>>,
    Query(request): Query<QueryLogsRequest>,
) -> ApiResult<QueryLogsResponse> {
    if state
        .tenant_registry
        .get_tenant(&request.tenant_id)
        .map_err(|err| internal_error(err))?
        .is_none()
    {
        return Err(not_found("tenant_not_found", "tenant not registered"));
    }

    let filter = LogFilter {
        start_time: request.start_time.clone(),
        end_time: request.end_time.clone(),
        decision: request.decision.clone(),
        protocol: request.protocol.clone(),
        limit: request.limit.or(Some(100)),
    };

    let logs = state
        .database
        .query_logs(&request.tenant_id, &filter)
        .map_err(internal_error)?;

    Ok(Json(QueryLogsResponse { logs }))
}

pub async fn get_unuploaded_logs(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<UnuploadedQuery>,
) -> ApiResult<QueryLogsResponse> {
    if state
        .tenant_registry
        .get_tenant(&query.tenant_id)
        .map_err(|err| internal_error(err))?
        .is_none()
    {
        return Err(not_found("tenant_not_found", "tenant not registered"));
    }

    let limit = query.limit.unwrap_or(state.config.upload_batch_size);
    let logs = state
        .database
        .get_unuploaded_logs(&query.tenant_id, limit)
        .map_err(|err| internal_error(err))?;

    Ok(Json(QueryLogsResponse { logs }))
}

pub async fn mark_uploaded(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MarkUploadedRequest>,
) -> ApiResult<QueryLogsResponse> {
    if request.log_ids.is_empty() {
        return Err(bad_request(
            "invalid_request",
            "log_ids cannot be empty",
        ));
    }

    state
        .database
        .mark_logs_uploaded(&request.tenant_id, &request.log_ids)
        .map_err(|err| internal_error(err))?;

    Ok(Json(QueryLogsResponse { logs: vec![] }))
}

pub async fn create_tenant(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<TenantRequest>,
) -> ApiResult<TenantResponse> {
    if request.tenant_id.trim().is_empty() {
        return Err(bad_request("invalid_tenant_id", "tenant_id cannot be empty"));
    }

    let now = Utc::now().to_rfc3339();
    let record = TenantRecord {
        tenant_id: request.tenant_id.clone(),
        name: request.name.clone(),
        status: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
        config: request.config.clone(),
    };

    state
        .tenant_registry
        .create_tenant(&record)
        .map_err(|err| internal_error(err))?;

    info!(tenant_id = %request.tenant_id, "registered tenant");

    Ok(Json(TenantResponse::from(record)))
}

pub async fn get_tenant(
    State(state): State<Arc<ApiState>>,
    Path(tenant_id): Path<String>,
) -> ApiResult<TenantResponse> {
    let tenant = state
        .tenant_registry
        .get_tenant(&tenant_id)
        .map_err(|err| internal_error(err))?;

    match tenant {
        Some(record) => Ok(Json(TenantResponse::from(record))),
        None => Err(not_found("tenant_not_found", "tenant not registered")),
    }
}

pub async fn update_tenant(
    State(state): State<Arc<ApiState>>,
    Path(tenant_id): Path<String>,
    Json(request): Json<UpdateTenantRequest>,
) -> ApiResult<TenantResponse> {
    let existing = state
        .tenant_registry
        .get_tenant(&tenant_id)
        .map_err(|err| internal_error(err))?;

    let mut record = match existing {
        Some(record) => record,
        None => return Err(not_found("tenant_not_found", "tenant not registered")),
    };

    if let Some(name) = request.name {
        record.name = name;
    }

    if let Some(status) = request.status.clone() {
        // Validate status
        if !matches!(status.as_str(), "active" | "suspended" | "deleted") {
            return Err(bad_request(
                "invalid_status",
                &format!("status must be one of: active, suspended, deleted; got: {}", status),
            ));
        }
        record.status = status;
    }

    if let Some(config) = request.config {
        record.config = Some(config);
    }

    record.updated_at = Utc::now().to_rfc3339();

    state
        .tenant_registry
        .update_tenant(&record)
        .map_err(|err| internal_error(err))?;

    info!(tenant_id = %tenant_id, "updated tenant");

    Ok(Json(TenantResponse::from(record)))
}

pub async fn delete_tenant(
    State(state): State<Arc<ApiState>>,
    Path(tenant_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let existing = state
        .tenant_registry
        .get_tenant(&tenant_id)
        .map_err(|err| internal_error(err))?;

    let mut record = match existing {
        Some(record) => record,
        None => return Err(not_found("tenant_not_found", "tenant not registered")),
    };

    // Soft delete by setting status to "deleted"
    record.status = "deleted".to_string();
    record.updated_at = Utc::now().to_rfc3339();

    state
        .tenant_registry
        .update_tenant(&record)
        .map_err(|err| internal_error(err))?;

    info!(tenant_id = %tenant_id, "deleted tenant (soft delete)");

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "tenant_id": tenant_id
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListTenantsQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PolicyBundlesQuery {
    pub tenant_id: String,
}

pub async fn list_tenants(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ListTenantsQuery>,
) -> ApiResult<Vec<TenantResponse>> {
    let tenants = state
        .tenant_registry
        .list_tenants(query.status.as_deref())
        .map_err(internal_error)?;

    Ok(Json(
        tenants
            .into_iter()
            .map(TenantResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn create_policy_bundle(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<PolicyBundleRecord>,
) -> ApiResult<PolicyBundleRecord> {
    if state
        .tenant_registry
        .get_tenant(&request.tenant_id)
        .map_err(|err| internal_error(err))?
        .is_none()
    {
        return Err(not_found("tenant_not_found", "tenant not registered"));
    }

    state
        .bundle_store
        .store_bundle(&request)
        .map_err(|err| internal_error(err))?;

    let stored = state
        .bundle_store
        .get_bundle(&request.bundle_id)
        .map_err(|err| internal_error(err))?
        .ok_or_else(|| internal_error("failed to load stored bundle"))?;

    info!(
        tenant_id = %stored.tenant_id,
        bundle_id = %stored.bundle_id,
        version = %stored.version,
        "stored policy bundle via API"
    );

    Ok(Json(stored))
}

pub async fn list_policy_bundles(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PolicyBundlesQuery>,
) -> ApiResult<Vec<PolicyBundleRecord>> {
    if state
        .tenant_registry
        .get_tenant(&query.tenant_id)
        .map_err(|err| internal_error(err))?
        .is_none()
    {
        return Err(not_found("tenant_not_found", "tenant not registered"));
    }

    let bundles = state
        .bundle_store
        .list_bundles(&query.tenant_id)
        .map_err(|err| internal_error(err))?;

    Ok(Json(bundles))
}

pub async fn get_policy_bundle(
    State(state): State<Arc<ApiState>>,
    Path(bundle_id): Path<String>,
) -> ApiResult<PolicyBundleRecord> {
    let bundle = state
        .bundle_store
        .get_bundle(&bundle_id)
        .map_err(|err| internal_error(err))?;

    match bundle {
        Some(record) => Ok(Json(record)),
        None => Err(not_found("bundle_not_found", "policy bundle not found")),
    }
}

pub async fn activate_policy_bundle(
    State(state): State<Arc<ApiState>>,
    Path(bundle_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    state
        .bundle_store
        .activate_bundle(&bundle_id)
        .map_err(|err| internal_error(err))?;

    info!(bundle_id = %bundle_id, "activated policy bundle via API");

    Ok(Json(serde_json::json!({
        "status": "activated",
        "bundle_id": bundle_id
    })))
}

pub async fn archive_policy_bundle(
    State(state): State<Arc<ApiState>>,
    Path(bundle_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    state
        .bundle_store
        .archive_bundle(&bundle_id)
        .map_err(|err| internal_error(err))?;

    info!(bundle_id = %bundle_id, "archived policy bundle via API");

    Ok(Json(serde_json::json!({
        "status": "archived",
        "bundle_id": bundle_id
    })))
}

pub async fn health_check() -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "audit-store"
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
    error!(error = %err, "internal error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal server error".to_string(),
            code: "internal_error".to_string(),
            details: Some(serde_json::json!({ "message": err.to_string() })),
        }),
    )
}
