use super::{default_tenant_config, random_tenant_id, TestHarness};
use anyhow::Result;
use serde_json::json;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tokio::time::{sleep, Duration};

async fn setup_tenant_with_policy(harness: &mut TestHarness) -> Result<String> {
    let tenant_id = random_tenant_id("tenant-offline");
    harness
        .create_test_tenant(&tenant_id, &default_tenant_config())
        .await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            &format!(
                r#"
allow read sensor_data if subject.tenant_id == "{tenant}"
"#,
                tenant = tenant_id
            ),
        )
        .await?;
    Ok(tenant_id)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_enforcer_offline_operation() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = setup_tenant_with_policy(&mut harness).await?;

    harness.stop_service("audit-store").await.ok();
    harness.stop_service("quota-tracker").await.ok();

    let url = format!(
        "http://127.0.0.1:{}/v1/data/tenants/{tenant_id}/allow",
        harness.ports().enforcer
    );
    let mut eval_times_ns = Vec::new();
    for _ in 0..10 {
        let response = harness
            .http_client()
            .post(&url)
            .json(&json!({
                "input": {
                    "subject": { "tenant_id": tenant_id },
                    "resource": { "type": "sensor_data" },
                    "action": "read",
                    "environment": {}
                }
            }))
            .send()
            .await?;

        assert_eq!(response.status(), 200, "Enforcer should return 200 while offline");

        // Parse response and extract evaluation_time_ns from metrics
        let body: serde_json::Value = response.json().await?;
        if let Some(metrics) = body.get("metrics") {
            if let Some(eval_time) = metrics.get("eval_duration_micros").and_then(|v| v.as_u64()) {
                // Convert micros to nanos
                eval_times_ns.push(eval_time * 1000);
            }
        }
    }

    assert!(!eval_times_ns.is_empty(), "Should have collected evaluation metrics");

    // Compute p99 (for 10 samples, p99 is the max value)
    let p99_ns = *eval_times_ns.iter().max().unwrap_or(&0);
    let p99_duration = Duration::from_nanos(p99_ns);

    assert!(
        p99_ns < 2_000_000,
        "expected p99 evaluation time <2ms (2,000,000ns), got {}ns ({:?})",
        p99_ns,
        p99_duration
    );

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires upload endpoint simulation"]
async fn test_audit_deferred_upload() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = setup_tenant_with_policy(&mut harness).await?;

    let audit_url = format!(
        "http://127.0.0.1:{}/api/audit/logs",
        harness.ports().audit_store
    );
    harness
        .http_client()
        .post(&audit_url)
        .json(&json!({
            "tenant_id": tenant_id,
            "decision": "allow",
            "protocol": "http",
            "resource": "sensor_data",
            "uploaded": 0
        }))
        .send()
        .await?;

    harness.stop_service("audit-store").await?;
    sleep(Duration::from_secs(1)).await;
    harness.restart_service("audit-store").await?;

    let query_url = format!(
        "http://127.0.0.1:{}/api/audit/logs?tenant_id={tenant_id}",
        harness.ports().audit_store
    );
    let response = harness.http_client().get(query_url).send().await?;
    let body: serde_json::Value = response.json().await?;
    let first = body["logs"]
        .as_array()
        .and_then(|arr| arr.first())
        .cloned()
        .unwrap_or_default();
    assert_eq!(first["uploaded"], json!(0));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_quota_persistence_on_restart() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = setup_tenant_with_policy(&mut harness).await?;

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

    harness.stop_service("quota-tracker").await?;
    sleep(Duration::from_millis(500)).await;
    harness.restart_service("quota-tracker").await?;

    let get_url = format!(
        "http://127.0.0.1:{}/api/quota/{tenant_id}",
        harness.ports().quota_tracker
    );
    let response = harness.http_client().get(get_url).send().await?;
    let metrics: serde_json::Value = response.json().await?;
    assert_eq!(metrics["metrics"]["message_count"], json!(5));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_proxy_continues_with_enforcer_down() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = setup_tenant_with_policy(&mut harness).await?;

    let proxy_url = format!(
        "http://127.0.0.1:{}/health",
        harness.ports().proxy_http
    );
    harness.http_client().get(&proxy_url).send().await?.error_for_status()?;

    harness.stop_service("enforcer").await?;

    let proxy_req_url = format!(
        "http://127.0.0.1:{}/api/test",
        harness.ports().proxy_http
    );
    let response = harness
        .http_client()
        .get(proxy_req_url)
        .header("X-Tenant-ID", tenant_id)
        .send()
        .await?;
    assert_eq!(response.status(), 503);

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires filesystem manipulation"]
async fn test_policy_bundle_caching() -> Result<()> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_id = setup_tenant_with_policy(&mut harness).await?;

    #[cfg(unix)]
    {
        let tenant_bundle_dir = harness.enforcer_bundle_dir().join(&tenant_id);
        tokio::fs::set_permissions(
            &tenant_bundle_dir,
            std::fs::Permissions::from_mode(0o555),
        )
        .await?;
    }

    let url = format!(
        "http://127.0.0.1:{}/v1/data/tenants/{tenant_id}/allow",
        harness.ports().enforcer
    );
    let response = harness
        .http_client()
        .post(&url)
        .json(&json!({
            "input": {
                "subject": { "tenant_id": tenant_id },
                "resource": { "type": "sensor_data" },
                "action": "read",
                "environment": {}
            }
        }))
        .send()
        .await?;
    response.error_for_status()?;

    harness.cleanup().await?;
    Ok(())
}
