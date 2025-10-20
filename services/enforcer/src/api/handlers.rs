use std::{sync::Arc, time::Instant};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    policy::{PolicyError, PolicyManager},
    tenant::{validate_tenant_id_format, validate_tenant_match, TenantValidationError},
};

use super::types::{
    DecisionEvent, ErrorResponse, EvaluationMetrics, PolicyQueryRequest, PolicyQueryResponse,
};

#[instrument(skip(policy_manager, request), fields(tenant_id = %tenant_id))]
pub async fn query_policy(
    Path(tenant_id): Path<String>,
    State((policy_manager, event_tx)): State<(
        Arc<PolicyManager>,
        Arc<broadcast::Sender<DecisionEvent>>,
    )>,
    Json(request): Json<PolicyQueryRequest>,
) -> Result<Json<PolicyQueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_tenant_id_format(&tenant_id).map_err(|err| map_validation_error(err))?;
    let input = request.input;
    validate_tenant_match(&tenant_id, &input).map_err(|err| map_validation_error(err))?;

    let eval_start = Instant::now();
    let decision = policy_manager
        .evaluate(&tenant_id, input.clone())
        .await
        .map_err(|err| map_policy_error(err))?;
    let eval_duration = eval_start.elapsed();

    info!(
        tenant = %tenant_id,
        elapsed_us = eval_duration.as_micros(),
        "policy query handled"
    );

    let metrics = EvaluationMetrics {
        eval_duration_micros: eval_duration.as_micros() as u64,
        tenant_id: tenant_id.clone(),
    };

    let event = DecisionEvent {
        event_id: Uuid::new_v4().to_string(),
        tenant_id: tenant_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        decision: decision.clone(),
        input,
        metrics: metrics.clone(),
    };

    let _ = event_tx.send(event);

    Ok(Json(PolicyQueryResponse {
        result: decision,
        metrics: Some(metrics),
    }))
}

#[instrument(skip(policy_manager))]
pub async fn health_check(
    State((policy_manager, _event_tx)): State<(
        Arc<PolicyManager>,
        Arc<broadcast::Sender<DecisionEvent>>,
    )>,
) -> Json<Value> {
    let tenant_count = policy_manager.list_tenants().len();
    Json(json!({
        "status": "healthy",
        "service": "edge-policy-enforcer",
        "tenant_count": tenant_count
    }))
}

#[instrument(skip(policy_manager), fields(tenant_id = %tenant_id))]
pub async fn reload_tenant(
    Path(tenant_id): Path<String>,
    State((policy_manager, _event_tx)): State<(
        Arc<PolicyManager>,
        Arc<broadcast::Sender<DecisionEvent>>,
    )>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_tenant_id_format(&tenant_id).map_err(|err| map_validation_error(err))?;

    policy_manager
        .reload_tenant(&tenant_id)
        .map_err(|err| map_policy_error(err))?;

    info!(tenant = %tenant_id, "tenant bundle reloaded");

    Ok(Json(json!({
        "status": "ok",
        "tenant_id": tenant_id
    })))
}

fn map_validation_error(err: TenantValidationError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code) = match err {
        TenantValidationError::Mismatch { .. } => (StatusCode::FORBIDDEN, "TENANT_MISMATCH"),
        TenantValidationError::MissingInputTenant => (StatusCode::BAD_REQUEST, "MISSING_TENANT"),
        TenantValidationError::InvalidTenantId(_) => (StatusCode::BAD_REQUEST, "INVALID_TENANT"),
    };

    (
        status,
        Json(ErrorResponse {
            error: err.to_string(),
            code: code.to_string(),
            details: None,
        }),
    )
}

fn map_policy_error(err: PolicyError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        PolicyError::TenantNotFound(tenant_id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("tenant '{}' not found", tenant_id),
                code: "TENANT_NOT_FOUND".to_string(),
                details: None,
            }),
        ),
        PolicyError::EvaluationFailed { tenant_id, source } => {
            error!(
                tenant = %tenant_id,
                error = ?source,
                "policy evaluation failed"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "policy evaluation failed".to_string(),
                    code: "EVALUATION_ERROR".to_string(),
                    details: Some(json!({ "tenant_id": tenant_id })),
                }),
            )
        }
        PolicyError::BundleLoadError { tenant_id, source } => {
            error!(
                tenant = %tenant_id,
                error = ?source,
                "tenant bundle load failed"
            );
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "tenant bundle could not be loaded".to_string(),
                    code: "TENANT_BUNDLE_ERROR".to_string(),
                    details: Some(json!({ "tenant_id": tenant_id })),
                }),
            )
        }
        PolicyError::InvalidPolicy { tenant_id, reason } => {
            error!(
                tenant = %tenant_id,
                %reason,
                "invalid policy encountered"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "invalid policy".to_string(),
                    code: "INVALID_POLICY".to_string(),
                    details: Some(json!({ "tenant_id": tenant_id, "reason": reason })),
                }),
            )
        }
    }
}
