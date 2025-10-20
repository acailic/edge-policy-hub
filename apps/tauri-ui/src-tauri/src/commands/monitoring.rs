use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::{config::ServiceConfig, error::CommandError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub log_id: String,
    pub tenant_id: String,
    pub timestamp: String,
    pub decision: String,
    pub protocol: String,
    pub subject: Value,
    pub action: String,
    pub resource: Value,
    pub environment: Value,
    pub policy_version: Option<u32>,
    pub reason: Option<String>,
    pub signature: String,
    pub uploaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaMetrics {
    pub tenant_id: String,
    pub message_count: u64,
    pub bytes_sent: u64,
    pub message_limit: u64,
    pub bandwidth_limit_bytes: u64,
    pub last_reset: DateTime<Utc>,
    pub period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub exceeded: bool,
    pub quota_type: Option<String>,
    pub limit: Option<u64>,
    pub current: Option<u64>,
    pub warning_threshold_reached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryLogsRequest {
    tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct QueryLogsResponse {
    logs: Vec<AuditLogEntry>,
}

#[derive(Debug, Deserialize)]
struct GetQuotaResponse {
    metrics: QuotaMetrics,
}

#[derive(Debug, Serialize)]
struct CheckQuotaRequest {
    tenant_id: String,
}

#[derive(Debug, Deserialize)]
struct CheckQuotaResponse {
    exceeded: bool,
    quota_type: Option<String>,
    limit: Option<u64>,
    current: Option<u64>,
}

#[tauri::command]
pub async fn query_audit_logs(
    tenant_id: String,
    start_time: Option<String>,
    end_time: Option<String>,
    decision: Option<String>,
    protocol: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AuditLogEntry>, String> {
    query_audit_logs_impl(tenant_id, start_time, end_time, decision, protocol, limit)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_quota_metrics(tenant_id: String) -> Result<QuotaMetrics, String> {
    get_quota_metrics_impl(&tenant_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_all_quota_metrics() -> Result<Vec<QuotaMetrics>, String> {
    list_all_quota_metrics_impl()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn check_quota_status(tenant_id: String) -> Result<QuotaStatus, String> {
    check_quota_status_impl(&tenant_id)
        .await
        .map_err(|err| err.to_string())
}

async fn query_audit_logs_impl(
    tenant_id: String,
    start_time: Option<String>,
    end_time: Option<String>,
    decision: Option<String>,
    protocol: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AuditLogEntry>, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.audit_store_url, "api/audit/logs")?;

    let payload = QueryLogsRequest {
        tenant_id: tenant_id.clone(),
        start_time,
        end_time,
        decision,
        protocol,
        limit,
    };

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let logs = response
        .json::<QueryLogsResponse>()
        .await
        .map_err(CommandError::from)?
        .logs;

    info!(
      tenant_id = %tenant_id,
      count = logs.len(),
      "fetched audit logs via Tauri command"
    );

    Ok(logs)
}

async fn get_quota_metrics_impl(tenant_id: &str) -> Result<QuotaMetrics, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.quota_tracker_url, &format!("api/quota/{tenant_id}"))?;

    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(CommandError::NotFound(format!(
            "quota metrics for tenant `{tenant_id}` not found"
        )));
    }

    if !response.status().is_success() {
        return Err(map_api_error(response, "quota-tracker").await);
    }

    let metrics = response
        .json::<GetQuotaResponse>()
        .await
        .map_err(CommandError::from)?
        .metrics;

    info!(
      tenant_id = %tenant_id,
      message_count = metrics.message_count,
      bytes_sent = metrics.bytes_sent,
      "retrieved quota metrics via Tauri command"
    );

    Ok(metrics)
}

async fn list_all_quota_metrics_impl() -> Result<Vec<QuotaMetrics>, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.quota_tracker_url, "api/quota")?;

    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "quota-tracker").await);
    }

    let metrics = response
        .json::<Vec<QuotaMetrics>>()
        .await
        .map_err(CommandError::from)?;

    info!(
        tenants = metrics.len(),
        "retrieved aggregated quota metrics via Tauri command"
    );

    Ok(metrics)
}

async fn check_quota_status_impl(tenant_id: &str) -> Result<QuotaStatus, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.quota_tracker_url, "api/quota/check")?;

    let payload = CheckQuotaRequest {
        tenant_id: tenant_id.to_string(),
    };

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(CommandError::NotFound(format!(
            "quota status for tenant `{tenant_id}` not found"
        )));
    }

    if !response.status().is_success() {
        return Err(map_api_error(response, "quota-tracker").await);
    }

    let body = response
        .json::<CheckQuotaResponse>()
        .await
        .map_err(CommandError::from)?;

    let warning_threshold_reached = body.exceeded
        || body
            .limit
            .zip(body.current)
            .map(|(limit, current)| {
                if limit == 0 {
                    false
                } else {
                    let percentage = (current as f64 / limit as f64) * 100.0;
                    percentage >= 80.0
                }
            })
            .unwrap_or(false);

    let status = QuotaStatus {
        exceeded: body.exceeded,
        quota_type: body.quota_type.clone(),
        limit: body.limit,
        current: body.current,
        warning_threshold_reached,
    };

    info!(
      tenant_id = %tenant_id,
      exceeded = status.exceeded,
      warning = status.warning_threshold_reached,
      quota_type = %status.quota_type.clone().unwrap_or_else(|| "unknown".to_string()),
      "checked quota status via Tauri command"
    );

    Ok(status)
}

fn setup_client() -> Result<(ServiceConfig, Client), CommandError> {
    let config =
        ServiceConfig::from_env().map_err(|err| CommandError::ValidationError(err.to_string()))?;

    let client = Client::builder()
        .timeout(Duration::from_secs(config.request_timeout_secs))
        .build()
        .map_err(|err| CommandError::NetworkError(err.to_string()))?;

    Ok((config, client))
}

fn build_url(base: &str, path: &str) -> Result<Url, CommandError> {
    let mut url = Url::parse(base).map_err(|err| CommandError::ValidationError(err.to_string()))?;

    url.path_segments_mut()
        .map_err(|_| CommandError::ValidationError("invalid base url".to_string()))?
        .extend(path.split('/').filter(|segment| !segment.is_empty()));

    Ok(url)
}

async fn map_api_error(response: reqwest::Response, service: &str) -> CommandError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| {
                    value
                        .get("message")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                })
        })
        .unwrap_or_else(|| {
            if body.is_empty() {
                "unknown error".to_string()
            } else {
                body.clone()
            }
        });

    if status == StatusCode::NOT_FOUND {
        CommandError::NotFound(message)
    } else {
        warn!(
          service = %service,
          status = %status,
          message = %message,
          "received error response from service"
        );
        CommandError::ApiError {
            service: service.to_string(),
            status: status.as_u16(),
            message,
        }
    }
}
