use std::time::Duration;

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{config::ServiceConfig, error::CommandError};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TenantConfig {
    pub quotas: Option<TenantQuotas>,
    pub features: Option<TenantFeatures>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TenantQuotas {
    pub message_limit: u64,
    pub bandwidth_limit_gb: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TenantFeatures {
    pub data_residency: Vec<String>,
    pub pii_redaction: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tenant {
    pub tenant_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub config: Option<TenantConfig>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponseBody {
    error: Option<String>,
    code: Option<String>,
    details: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AuditStoreTenant {
    tenant_id: String,
    name: String,
    status: String,
    created_at: String,
    updated_at: String,
    config: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_id: String,
    pub name: String,
    pub config: Option<TenantConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub config: Option<TenantConfig>,
}

#[derive(Debug, Serialize)]
struct CreateTenantPayload<'a> {
    tenant_id: &'a str,
    name: &'a str,
    config: Option<Value>,
}

#[derive(Debug, Serialize)]
struct UpdateTenantPayload {
    name: Option<String>,
    status: Option<String>,
    config: Option<Value>,
}

#[derive(Debug, Serialize)]
struct SetLimitsRequest<'a> {
    tenant_id: &'a str,
    message_limit: u64,
    bandwidth_limit_gb: f64,
}

#[tauri::command]
pub async fn list_tenants(status_filter: Option<String>) -> Result<Vec<Tenant>, String> {
    list_tenants_impl(status_filter)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_tenant(tenant_id: String) -> Result<Tenant, String> {
    get_tenant_impl(&tenant_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_tenant(request: CreateTenantRequest) -> Result<Tenant, String> {
    create_tenant_impl(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_tenant(
    tenant_id: String,
    request: UpdateTenantRequest,
) -> Result<Tenant, String> {
    update_tenant_impl(&tenant_id, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_tenant(tenant_id: String) -> Result<(), String> {
    delete_tenant_impl(&tenant_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn set_quota_limits(
    tenant_id: String,
    message_limit: u64,
    bandwidth_limit_gb: f64,
) -> Result<(), String> {
    set_quota_limits_impl(&tenant_id, message_limit, bandwidth_limit_gb)
        .await
        .map_err(|err| err.to_string())
}

async fn list_tenants_impl(status_filter: Option<String>) -> Result<Vec<Tenant>, CommandError> {
    let (config, client) = setup_client()?;
    let mut url = build_url(&config.audit_store_url, "api/tenants")?;

    if let Some(status) = status_filter.filter(|value| !value.is_empty()) {
        url.query_pairs_mut().append_pair("status", &status);
    }

    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let tenants = response
        .json::<Vec<AuditStoreTenant>>()
        .await
        .map_err(CommandError::from)?
        .into_iter()
        .map(convert_tenant)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tenants)
}

async fn get_tenant_impl(tenant_id: &str) -> Result<Tenant, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.audit_store_url, &format!("api/tenants/{tenant_id}"))?;
    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(CommandError::NotFound(format!(
            "tenant `{tenant_id}` not found"
        )));
    }

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let tenant = response
        .json::<AuditStoreTenant>()
        .await
        .map_err(CommandError::from)
        .and_then(convert_tenant)?;

    Ok(tenant)
}

async fn create_tenant_impl(request: CreateTenantRequest) -> Result<Tenant, CommandError> {
    validate_tenant_id(&request.tenant_id)?;

    let (config, client) = setup_client()?;
    let url = build_url(&config.audit_store_url, "api/tenants")?;

    let payload = CreateTenantPayload {
        tenant_id: &request.tenant_id,
        name: &request.name,
        config: request.config.as_ref().map(|config| json!(config)),
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

    let created = response
        .json::<AuditStoreTenant>()
        .await
        .map_err(CommandError::from)
        .and_then(convert_tenant)?;

    if let Some(config_payload) = created.config.clone() {
        if let Some(quotas) = config_payload.quotas {
            set_quota_limits_impl(
                &created.tenant_id,
                quotas.message_limit,
                quotas.bandwidth_limit_gb,
            )
            .await?;
        }
    }

    info!(tenant_id = %created.tenant_id, "created tenant via Tauri command");

    Ok(created)
}

async fn update_tenant_impl(
    tenant_id: &str,
    request: UpdateTenantRequest,
) -> Result<Tenant, CommandError> {
    let (config, client) = setup_client()?;
    let existing = get_tenant_impl(tenant_id).await?;

    let merged_config = request.config.clone().or(existing.config.clone());

    let url = build_url(&config.audit_store_url, &format!("api/tenants/{tenant_id}"))?;

    let payload = UpdateTenantPayload {
        name: request.name.clone(),
        status: request.status.clone(),
        config: merged_config.as_ref().map(|cfg| json!(cfg)),
    };

    let response = client
        .put(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let updated = response
        .json::<AuditStoreTenant>()
        .await
        .map_err(CommandError::from)
        .and_then(convert_tenant)?;

    if let Some(config_payload) = request.config {
        if let Some(quotas) = config_payload.quotas {
            let previous_quotas = existing.config.and_then(|cfg| cfg.quotas);
            let changed = previous_quotas
                .map(|prev| {
                    prev.message_limit != quotas.message_limit
                        || (prev.bandwidth_limit_gb - quotas.bandwidth_limit_gb).abs()
                            > f64::EPSILON
                })
                .unwrap_or(true);

            if changed {
                set_quota_limits_impl(tenant_id, quotas.message_limit, quotas.bandwidth_limit_gb)
                    .await?;
            }
        }
    }

    info!(tenant_id = %tenant_id, "updated tenant via Tauri command");

    Ok(updated)
}

async fn delete_tenant_impl(tenant_id: &str) -> Result<(), CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.audit_store_url, &format!("api/tenants/{tenant_id}"))?;

    let response = client
        .delete(url)
        .send()
        .await
        .map_err(CommandError::from)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(CommandError::NotFound(format!(
            "tenant `{tenant_id}` not found"
        )));
    }

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    info!(tenant_id = %tenant_id, "deleted tenant via Tauri command");

    Ok(())
}

async fn set_quota_limits_impl(
    tenant_id: &str,
    message_limit: u64,
    bandwidth_limit_gb: f64,
) -> Result<(), CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(&config.quota_tracker_url, "api/quota/limits")?;

    let payload = SetLimitsRequest {
        tenant_id,
        message_limit,
        bandwidth_limit_gb,
    };

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "quota-tracker").await);
    }

    info!(
      tenant_id = %tenant_id,
      "updated quota limits via Tauri command"
    );

    Ok(())
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

    let message = serde_json::from_str::<ErrorResponseBody>(&body)
        .ok()
        .and_then(|parsed| parsed.error)
        .or_else(|| {
            serde_json::from_str::<Value>(&body).ok().and_then(|value| {
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

fn convert_tenant(source: AuditStoreTenant) -> Result<Tenant, CommandError> {
    let config = match source.config {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_json::from_value::<TenantConfig>(value).map_err(CommandError::from)?)
            }
        }
        None => None,
    };

    Ok(Tenant {
        tenant_id: source.tenant_id,
        name: source.name,
        status: source.status,
        created_at: source.created_at,
        updated_at: source.updated_at,
        config,
    })
}

fn validate_tenant_id(tenant_id: &str) -> Result<(), CommandError> {
    if tenant_id.trim().is_empty() {
        return Err(CommandError::ValidationError(
            "tenant_id cannot be empty".to_string(),
        ));
    }

    if tenant_id.len() > 64 {
        return Err(CommandError::ValidationError(
            "tenant_id must be at most 64 characters".to_string(),
        ));
    }

    if !tenant_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(CommandError::ValidationError(
            "tenant_id may contain letters, numbers, underscores, and hyphens only".to_string(),
        ));
    }

    Ok(())
}
