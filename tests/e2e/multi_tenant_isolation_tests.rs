use super::{default_tenant_config, random_tenant_id, TestHarness};
use anyhow::{Context, Result};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

async fn setup_harness_with_two_tenants() -> Result<(TestHarness, String, String)> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;
    let tenant_a = random_tenant_id("tenant-a");
    let tenant_b = random_tenant_id("tenant-b");
    harness
        .create_test_tenant(&tenant_a, &default_tenant_config())
        .await?;
    harness
        .create_test_tenant(&tenant_b, &default_tenant_config())
        .await?;
    Ok((harness, tenant_a, tenant_b))
}

async fn query_enforcer(
    harness: &TestHarness,
    tenant_id: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value> {
    let url = format!(
        "http://127.0.0.1:{}/v1/data/tenants/{tenant_id}/allow",
        harness.ports().enforcer
    );
    let response = harness
        .http_client()
        .post(url)
        .json(&json!({ "input": input }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("enforcer query failed: {status} {body}");
    }
    Ok(serde_json::from_str(&body)?)
}

async fn create_mqtt_client(
    harness: &TestHarness,
    tenant_id: &str,
    client_suffix: &str,
) -> Result<(AsyncClient, JoinHandle<()>)> {
    let mut options = MqttOptions::new(
        format!("{tenant_id}/{client_suffix}"),
        "127.0.0.1",
        harness.ports().mqtt_bridge,
    );
    options.set_keep_alive(Duration::from_secs(5));

    let (client, mut event_loop) = AsyncClient::new(options, 10);
    let handle = tokio::spawn(async move {
        while let Ok(notification) = event_loop.poll().await {
            tracing::debug!("MQTT notification: {:?}", notification);
        }
    });
    sleep(Duration::from_millis(100)).await;
    Ok((client, handle))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_policy_namespace_isolation() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;

    harness
        .deploy_test_policy(
            &tenant_a,
            &format!(
                r#"
allow read sensor_data if
  subject.tenant_id == "{tenant_a}"
"#,
            ),
        )
        .await?;

    harness
        .deploy_test_policy(
            &tenant_b,
            &format!(
                r#"
deny read sensor_data if
  subject.tenant_id == "{tenant_b}"
"#,
            ),
        )
        .await?;

    let allow = query_enforcer(
        &harness,
        &tenant_a,
        json!({
            "subject": { "tenant_id": tenant_a, "roles": ["operator"] },
            "resource": { "type": "sensor_data", "owner_tenant": tenant_a },
            "action": "read",
            "environment": {}
        }),
    )
    .await?;
    assert_eq!(allow["result"]["allow"], json!(true));

    let deny = query_enforcer(
        &harness,
        &tenant_b,
        json!({
            "subject": { "tenant_id": tenant_b, "roles": ["operator"] },
            "resource": { "type": "sensor_data", "owner_tenant": tenant_b },
            "action": "read",
            "environment": {}
        }),
    )
    .await?;
    assert_eq!(deny["result"]["allow"], json!(false));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_audit_log_isolation() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;

    let payload_a = json!({
        "tenant_id": tenant_a,
        "decision": "allow",
        "resource": "sensor_data",
        "protocol": "http",
        "attributes": { "subject": { "tenant_id": tenant_a } }
    });
    let payload_b = json!({
        "tenant_id": tenant_b,
        "decision": "deny",
        "resource": "sensor_data",
        "protocol": "mqtt",
        "attributes": { "subject": { "tenant_id": tenant_b } }
    });

    for payload in [&payload_a, &payload_b] {
        let url = format!(
            "http://127.0.0.1:{}/api/audit/logs",
            harness.ports().audit_store
        );
        harness.http_client().post(&url).json(payload).send().await?;
    }

    let url = format!(
        "http://127.0.0.1:{}/api/audit/logs?tenant_id={tenant_a}",
        harness.ports().audit_store
    );
    let response = harness.http_client().get(url).send().await?;
    let body: serde_json::Value = response.json().await?;
    let logs = body["logs"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();
    assert!(
        logs.iter()
            .all(|entry| entry["tenant_id"] == json!(tenant_a)),
        "expected only tenant_a logs"
    );

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_quota_isolation() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;
    let url = format!(
        "http://127.0.0.1:{}/api/quota/increment",
        harness.ports().quota_tracker
    );
    harness
        .http_client()
        .post(&url)
        .json(&json!({
            "tenant_id": tenant_a,
            "message_count": 100,
            "bytes_sent": 1024
        }))
        .send()
        .await?;

    let tenant_b_url = format!(
        "http://127.0.0.1:{}/api/quota/{tenant_b}",
        harness.ports().quota_tracker
    );
    let response = harness.http_client().get(&tenant_b_url).send().await?;
    let metrics: serde_json::Value = response.json().await?;
    assert_eq!(
        metrics["metrics"]["message_count"],
        json!(0),
        "tenant B quota should remain untouched"
    );

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_tenant_policy_access_denied() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;
    harness
        .deploy_test_policy(
            &tenant_a,
            &format!(
                r#"
allow read sensor_data if subject.tenant_id == "{tenant_a}"
"#
            ),
        )
        .await?;

    let response = query_enforcer(
        &harness,
        &tenant_b,
        json!({
            "subject": { "tenant_id": tenant_b, "impersonated_tenant": tenant_a },
            "resource": { "type": "sensor_data", "owner_tenant": tenant_a },
            "action": "read",
            "environment": {}
        }),
    )
    .await?;
    assert_eq!(response["result"]["allow"], json!(false));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_tenant_resource_access() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;
    harness
        .deploy_test_policy(
            &tenant_a,
            &format!(
                r#"
allow read sensor_data if
  subject.tenant_id == "{tenant_a}" and
  resource.owner_tenant == "{tenant_a}"
"#
            ),
        )
        .await?;

    let result = query_enforcer(
        &harness,
        &tenant_a,
        json!({
            "subject": { "tenant_id": tenant_b, "roles": ["operator"] },
            "resource": { "type": "sensor_data", "owner_tenant": tenant_a },
            "action": "read",
            "environment": {}
        }),
    )
    .await?;
    assert_eq!(result["result"]["allow"], json!(false));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_policy_bundle_isolation() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;

    let bundle_payload = json!({
        "tenant_id": tenant_a,
        "version": "1.0.0",
        "dsl": "allow read data if subject.tenant_id == \"tenant\"",
        "metadata": {}
    });

    let bundles_url = format!(
        "http://127.0.0.1:{}/api/bundles",
        harness.ports().audit_store
    );
    harness
        .http_client()
        .post(&bundles_url)
        .json(&bundle_payload)
        .send()
        .await?;

    let list_url = format!(
        "http://127.0.0.1:{}/api/bundles?tenant_id={tenant_b}",
        harness.ports().audit_store
    );
    let response = harness.http_client().get(list_url).send().await?;
    let body: serde_json::Value = response.json().await?;
    let bundles_empty = body["bundles"]
        .as_array()
        .map(|arr| arr.is_empty())
        .unwrap_or(true);
    assert!(bundles_empty, "tenant B should not see tenant A bundles");

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires full MQTT stack"]
async fn test_session_isolation_mqtt() -> Result<()> {
    let (mut harness, tenant_a, tenant_b) = setup_harness_with_two_tenants().await?;
    harness
        .deploy_test_policy(
            &tenant_a,
            r#"
allow publish gps_telemetry if subject.tenant_id == input.tenant_id
allow subscribe gps_telemetry if subject.tenant_id == input.tenant_id
"#,
        )
        .await?;
    harness
        .deploy_test_policy(
            &tenant_b,
            r#"
allow publish gps_telemetry if subject.tenant_id == input.tenant_id
allow subscribe gps_telemetry if subject.tenant_id == input.tenant_id
"#,
        )
        .await?;

    let (client_a, handle_a) = create_mqtt_client(&harness, &tenant_a, "device-a").await?;
    let (client_b, handle_b) = create_mqtt_client(&harness, &tenant_b, "device-b").await?;

    client_a
        .subscribe(format!("{tenant_a}/#"), QoS::AtLeastOnce)
        .await?;
    client_b
        .subscribe(format!("{tenant_b}/#"), QoS::AtLeastOnce)
        .await?;

    let cross = client_a
        .subscribe(format!("{tenant_b}/#"), QoS::AtLeastOnce)
        .await;
    assert!(cross.is_err(), "tenant A should not subscribe to tenant B");

    let cross_publish = client_b
        .publish(
            format!("{tenant_a}/sensors/temp"),
            QoS::AtLeastOnce,
            false,
            "{}",
        )
        .await;
    assert!(
        cross_publish.is_err(),
        "tenant B should not publish into tenant A namespace"
    );

    handle_a.abort();
    handle_b.abort();
    harness.cleanup().await?;
    Ok(())
}
