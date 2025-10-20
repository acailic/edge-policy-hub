use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use edge_policy_dsl::{compile_policy, PolicyDslError, PolicyMetadata};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{config::ServiceConfig, error::CommandError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationError {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilePolicyResponse {
    pub success: bool,
    pub rego: Option<String>,
    pub errors: Option<Vec<CompilationError>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPolicyResponse {
    pub allow: bool,
    pub redact: Option<Vec<String>>,
    pub reason: Option<String>,
    pub eval_duration_micros: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPolicyResponse {
    pub bundle_id: String,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundle {
    pub bundle_id: String,
    pub tenant_id: String,
    pub version: i64,
    pub rego_code: String,
    pub metadata: Option<Value>,
    pub status: String,
    pub created_at: String,
    pub activated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct PolicyBundlePayload {
    bundle_id: String,
    tenant_id: String,
    version: i64,
    rego_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    status: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    activated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct PolicyQueryRequest {
    input: Value,
}

#[derive(Debug, Deserialize)]
struct PolicyQueryResponse {
    result: PolicyDecision,
    metrics: Option<EvaluationMetrics>,
}

#[derive(Debug, Deserialize)]
struct PolicyDecision {
    allow: bool,
    #[serde(default)]
    redact: Option<Vec<String>>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvaluationMetrics {
    eval_duration_micros: u64,
}

#[derive(Debug, Deserialize)]
struct ErrorResponseBody {
    error: Option<String>,
    code: Option<String>,
    details: Option<Value>,
}

#[tauri::command]
pub fn compile_policy_dsl(
    source: String,
    tenant_id: String,
    metadata: Option<PolicyMetadata>,
) -> Result<CompilePolicyResponse, String> {
    info!(%tenant_id, "compiling policy via Tauri command");

    match compile_policy(&source, &tenant_id, metadata) {
        Ok(compiled) => {
            info!(
              %tenant_id,
              rego_size = compiled.rego.len(),
              "policy compilation succeeded"
            );
            Ok(CompilePolicyResponse {
                success: true,
                rego: Some(compiled.rego),
                errors: None,
            })
        }
        Err(err) => {
            warn!(%tenant_id, error = %err, "policy compilation failed");
            let errors = match err {
                PolicyDslError::ParseError { message, location } => {
                    vec![CompilationError {
                        message,
                        line: location.map(|loc| loc.0 as u32),
                        column: location.map(|loc| loc.1 as u32),
                        attribute: None,
                    }]
                }
                PolicyDslError::ValidationError { message, attribute } => {
                    vec![CompilationError {
                        message,
                        line: None,
                        column: None,
                        attribute,
                    }]
                }
                PolicyDslError::InvalidAttribute { path, reason } => {
                    vec![CompilationError {
                        message: format!("{reason}"),
                        line: None,
                        column: None,
                        attribute: Some(path),
                    }]
                }
                other => vec![CompilationError {
                    message: other.to_string(),
                    line: None,
                    column: None,
                    attribute: None,
                }],
            };

            Ok(CompilePolicyResponse {
                success: false,
                rego: None,
                errors: Some(errors),
            })
        }
    }
}

#[tauri::command]
pub async fn test_policy(tenant_id: String, input: Value) -> Result<TestPolicyResponse, String> {
    test_policy_impl(&tenant_id, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn deploy_policy(
    tenant_id: String,
    rego_code: String,
    metadata: Value,
    activate: bool,
) -> Result<DeployPolicyResponse, String> {
    deploy_policy_impl(&tenant_id, rego_code, metadata, activate)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_policy_bundles(tenant_id: String) -> Result<Vec<PolicyBundle>, String> {
    list_policy_bundles_impl(&tenant_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_policy_bundle(bundle_id: String) -> Result<PolicyBundle, String> {
    get_policy_bundle_impl(&bundle_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn activate_policy_bundle(bundle_id: String) -> Result<(), String> {
    activate_policy_bundle_impl(&bundle_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn rollback_policy(tenant_id: String, bundle_id: String) -> Result<(), String> {
    rollback_policy_impl(&tenant_id, &bundle_id)
        .await
        .map_err(|err| err.to_string())
}

async fn test_policy_impl(
    tenant_id: &str,
    input: Value,
) -> Result<TestPolicyResponse, CommandError> {
    let (config, client) = setup_client()?;
    let url = build_url(
        &config.enforcer_url,
        &format!("/v1/data/tenants/{tenant_id}/allow"),
    )?;

    let payload = PolicyQueryRequest { input };

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "enforcer").await);
    }

    let body: PolicyQueryResponse = response.json().await.map_err(CommandError::from)?;

    let decision = body.result;
    let metrics = body.metrics.map(|value| value.eval_duration_micros);

    info!(
      tenant_id = %tenant_id,
      allow = %decision.allow,
      "policy test executed via Tauri command"
    );

    Ok(TestPolicyResponse {
        allow: decision.allow,
        redact: decision.redact,
        reason: decision.reason,
        eval_duration_micros: metrics,
    })
}

async fn deploy_policy_impl(
    tenant_id: &str,
    rego_code: String,
    metadata: Value,
    activate: bool,
) -> Result<DeployPolicyResponse, CommandError> {
    if rego_code.trim().is_empty() {
        return Err(CommandError::ValidationError(
            "compiled Rego code is required before deployment".to_string(),
        ));
    }

    let (config, client) = setup_client()?;

    let bundle_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    // Always create bundle as draft first
    let payload = PolicyBundlePayload {
        bundle_id: bundle_id.clone(),
        tenant_id: tenant_id.to_string(),
        version: 0,
        rego_code: rego_code.clone(),
        metadata: if metadata.is_null() {
            None
        } else {
            Some(metadata.clone())
        },
        status: "draft".to_string(),
        created_at: created_at.clone(),
        activated_at: None,
    };

    let url = build_url(&config.audit_store_url, "/api/bundles")?;
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let bundle: PolicyBundle = response.json().await.map_err(CommandError::from)?;

    // Only write .rego file and reload enforcer if activating
    if activate {
        // Atomically activate the bundle (demotes previous active versions)
        invoke_activate_bundle(&client, &config, &bundle.bundle_id).await?;

        // Write the .rego file to enforcer's watched directory
        write_policy_bundle_file(
            &config.enforcer_bundles_dir,
            tenant_id,
            bundle.version,
            &bundle.rego_code,
        )
        .await?;

        // Trigger hot-reload
        reload_enforcer(&client, &config, tenant_id).await?;
    }

    info!(
      tenant_id = %tenant_id,
      bundle_id = %bundle.bundle_id,
      version = %bundle.version,
      activate = %activate,
      "deployed policy bundle via Tauri command"
    );

    Ok(DeployPolicyResponse {
        bundle_id: bundle.bundle_id,
        version: bundle.version,
    })
}

async fn list_policy_bundles_impl(tenant_id: &str) -> Result<Vec<PolicyBundle>, CommandError> {
    let (config, client) = setup_client()?;
    let mut url = build_url(&config.audit_store_url, "/api/bundles")?;
    url.query_pairs_mut().append_pair("tenant_id", tenant_id);

    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let mut bundles: Vec<PolicyBundle> = response.json().await.map_err(CommandError::from)?;
    bundles.sort_by(|a, b| b.version.cmp(&a.version));

    Ok(bundles)
}

async fn get_policy_bundle_impl(bundle_id: &str) -> Result<PolicyBundle, CommandError> {
    let (config, client) = setup_client()?;
    fetch_policy_bundle(&client, &config, bundle_id)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("policy bundle `{bundle_id}` not found")))
}

async fn activate_policy_bundle_impl(bundle_id: &str) -> Result<(), CommandError> {
    let (config, client) = setup_client()?;
    invoke_activate_bundle(&client, &config, bundle_id).await?;
    let bundle = fetch_policy_bundle(&client, &config, bundle_id)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("policy bundle `{bundle_id}` not found")))?;

    write_policy_bundle_file(
        &config.enforcer_bundles_dir,
        &bundle.tenant_id,
        bundle.version,
        &bundle.rego_code,
    )
    .await?;
    reload_enforcer(&client, &config, &bundle.tenant_id).await?;

    info!(
      tenant_id = %bundle.tenant_id,
      bundle_id = %bundle.bundle_id,
      version = %bundle.version,
      "activated policy bundle via Tauri command"
    );

    Ok(())
}

async fn rollback_policy_impl(tenant_id: &str, bundle_id: &str) -> Result<(), CommandError> {
    let (config, client) = setup_client()?;
    let bundle = fetch_policy_bundle(&client, &config, bundle_id)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("policy bundle `{bundle_id}` not found")))?;

    if bundle.tenant_id != tenant_id {
        return Err(CommandError::ValidationError(format!(
            "bundle `{bundle_id}` does not belong to tenant `{tenant_id}`"
        )));
    }

    invoke_activate_bundle(&client, &config, bundle_id).await?;

    write_policy_bundle_file(
        &config.enforcer_bundles_dir,
        tenant_id,
        bundle.version,
        &bundle.rego_code,
    )
    .await?;
    reload_enforcer(&client, &config, tenant_id).await?;

    info!(
      tenant_id = %tenant_id,
      bundle_id = %bundle_id,
      version = %bundle.version,
      "rolled back policy bundle via Tauri command"
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

async fn fetch_policy_bundle(
    client: &Client,
    config: &ServiceConfig,
    bundle_id: &str,
) -> Result<Option<PolicyBundle>, CommandError> {
    let url = build_url(
        &config.audit_store_url,
        &format!("/api/bundles/{bundle_id}"),
    )?;

    let response = client.get(url).send().await.map_err(CommandError::from)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    let bundle: PolicyBundle = response.json().await.map_err(CommandError::from)?;
    Ok(Some(bundle))
}

async fn invoke_activate_bundle(
    client: &Client,
    config: &ServiceConfig,
    bundle_id: &str,
) -> Result<(), CommandError> {
    let url = build_url(
        &config.audit_store_url,
        &format!("/api/bundles/{bundle_id}/activate"),
    )?;

    let response = client.post(url).send().await.map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "audit-store").await);
    }

    Ok(())
}

async fn write_policy_bundle_file(
    bundles_dir: &PathBuf,
    tenant_id: &str,
    version: i64,
    rego_code: &str,
) -> Result<(), CommandError> {
    let tenant_dir = bundles_dir.join(tenant_id);
    fs::create_dir_all(&tenant_dir)
        .await
        .map_err(|err| CommandError::ValidationError(err.to_string()))?;

    let file_name = format!("policy_v{version}.rego");
    let file_path = tenant_dir.join(file_name);

    fs::write(&file_path, rego_code)
        .await
        .map_err(|err| CommandError::ValidationError(err.to_string()))?;

    Ok(())
}

async fn reload_enforcer(
    client: &Client,
    config: &ServiceConfig,
    tenant_id: &str,
) -> Result<(), CommandError> {
    let url = build_url(
        &config.enforcer_url,
        &format!("/v1/tenants/{tenant_id}/reload"),
    )?;

    let response = client.post(url).send().await.map_err(CommandError::from)?;

    if !response.status().is_success() {
        return Err(map_api_error(response, "enforcer").await);
    }

    Ok(())
}
