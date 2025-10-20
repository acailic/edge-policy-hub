use super::{default_tenant_config, random_tenant_id, TestHarness};
use anyhow::Result;
use futures_util::StreamExt;
use serde_json::json;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::connect_async;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_tenant_creation_workflow() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;

    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;

    let url = format!(
        "http://127.0.0.1:{}/api/tenants/{tenant_id}",
        harness.ports().audit_store
    );
    let response = harness.http_client().get(url).send().await?;
    assert!(response.status().is_success());

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_policy_compilation_workflow() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;

    let policy = r#"
allow read sensor_data if
  subject.tenant_id == "tenant"
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;

    let rego_path = harness
        .enforcer_bundle_dir()
        .join(&tenant_id)
        .join("policy.rego");
    let rego_source = tokio::fs::read_to_string(rego_path).await?;
    assert!(
        rego_source.contains("package tenants"),
        "compiled Rego should exist"
    );

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_policy_deployment_workflow() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;

    let policy = r#"
allow read sensor_data if subject.tenant_id == "tenant"
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;

    let bundles_url = format!(
        "http://127.0.0.1:{}/api/bundles?tenant_id={tenant_id}",
        harness.ports().audit_store
    );
    let bundles = harness.http_client().get(bundles_url).send().await?;
    let body: serde_json::Value = bundles.json().await?;
    assert!(
        body["bundles"]
            .as_array()
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "bundle should exist after deploy"
    );

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_policy_test_simulator() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            &format!(
                r#"
allow read sensor_data if
  subject.tenant_id == "{tenant_id}"
"#
            ),
        )
        .await?;

    let url = format!(
        "http://127.0.0.1:{}/v1/data/tenants/{tenant_id}/allow",
        harness.ports().enforcer
    );
    let response = harness
        .http_client()
        .post(url)
        .json(&json!({
            "input": {
                "subject": { "tenant_id": tenant_id },
                "resource": { "type": "sensor_data", "owner_tenant": tenant_id },
                "action": "read",
                "environment": { "time": "2023-01-01T12:00:00Z" }
            }
        }))
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["result"]["allow"], json!(true));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_monitoring_dashboard_realtime() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            &format!(
                r#"
allow read sensor_data if subject.tenant_id == "{tenant_id}"
"#
            ),
        )
        .await?;

    let ws_url = format!(
        "ws://127.0.0.1:{}/v1/stream/decisions?tenant_id={tenant_id}",
        harness.ports().enforcer
    );
    let (mut ws, _) = connect_async(ws_url).await?;

    let query_url = format!(
        "http://127.0.0.1:{}/v1/data/tenants/{tenant_id}/allow",
        harness.ports().enforcer
    );
    harness
        .http_client()
        .post(query_url)
        .json(&json!({
            "input": {
                "subject": { "tenant_id": tenant_id },
                "resource": { "type": "sensor_data", "owner_tenant": tenant_id },
                "action": "read",
                "environment": {}
            }
        }))
        .send()
        .await?;

    let mut received = false;
    let mut stream = ws.next();
    if let Some(message) = stream.await {
        if let Ok(msg) = message {
            if msg.is_text() {
                received = true;
            }
        }
    }
    assert!(received, "expected decision event");

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Tauri WebDriver environment"]
async fn test_quota_gauge_updates() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = random_tenant_id("tenant-ui");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;

    let increment_url = format!(
        "http://127.0.0.1:{}/api/quota/increment",
        harness.ports().quota_tracker
    );
    harness
        .http_client()
        .post(&increment_url)
        .json(&json!({
            "tenant_id": tenant_id,
            "message_count": 5,
            "bytes_sent": 2048
        }))
        .send()
        .await?;

    sleep(Duration::from_secs(1)).await;

    let quota_url = format!(
        "http://127.0.0.1:{}/api/quota/{tenant_id}",
        harness.ports().quota_tracker
    );
    let response = harness.http_client().get(quota_url).send().await?;
    let metrics: serde_json::Value = response.json().await?;
    assert_eq!(metrics["metrics"]["message_count"], json!(5));

    harness.cleanup().await?;
    Ok(())
}
