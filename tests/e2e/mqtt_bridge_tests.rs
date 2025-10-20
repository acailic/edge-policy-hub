use super::{default_tenant_config, random_tenant_id, TestHarness};
use anyhow::{Context, Result};
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde_json::json;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

struct MqttClient {
    client: AsyncClient,
    _loop_handle: JoinHandle<()>,
}

async fn create_mqtt_client(
    harness: &TestHarness,
    tenant_id: &str,
    client_id: &str,
) -> Result<MqttClient> {
    let mut options = MqttOptions::new(
        format!("{tenant_id}/{client_id}"),
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
    Ok(MqttClient {
        client,
        _loop_handle: handle,
    })
}

async fn setup_mqtt_environment() -> Result<(TestHarness, String)> {
    let mut harness = TestHarness::new().await?;
    harness.start_all_services().await?;

    let tenant_id = random_tenant_id("tenant-mqtt");
    let mut config = default_tenant_config();
    config
        .as_object_mut()
        .unwrap()
        .entry("mqtt")
        .or_insert(json!({}))
        .as_object_mut()
        .unwrap()
        .insert("namespace".into(), json!(tenant_id.clone()));
    harness.create_test_tenant(&tenant_id, &config).await?;
    Ok((harness, tenant_id))
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_publish_policy_enforcement_allow() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    let policy = r#"
allow publish gps_telemetry if
  subject.tenant_id == input.tenant_id and
  input.topic == concat(subject.tenant_id, "/sensors/temp")
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;

    let client = create_mqtt_client(&harness, &tenant_id, "device-1").await?;
    client
        .client
        .publish(
            format!("{tenant_id}/sensors/temp"),
            QoS::AtLeastOnce,
            false,
            serde_json::to_vec(&json!({"value": 22.1 })).unwrap(),
        )
        .await
        .context("publishing MQTT message")?;

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_publish_policy_enforcement_deny() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    let policy = r#"
allow publish gps_telemetry if
  subject.tenant_id == input.tenant_id and
  environment.message_count < 2
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;
    let client = create_mqtt_client(&harness, &tenant_id, "device-2").await?;

    for _ in 0..2 {
        client
            .client
            .publish(
                format!("{tenant_id}/sensors/temp"),
                QoS::AtLeastOnce,
                false,
                "{}",
            )
            .await?;
    }

    let err = client
        .client
        .publish(
            format!("{tenant_id}/sensors/temp"),
            QoS::AtLeastOnce,
            false,
            "{}",
        )
        .await;
    assert!(err.is_err());

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_topic_namespace_validation() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            r#"
allow publish gps_telemetry if subject.tenant_id == input.tenant_id
"#,
        )
        .await?;

    let client = create_mqtt_client(&harness, &tenant_id, "device-namespace").await?;
    let result = client
        .client
        .publish(
            "other-tenant/sensors/temp",
            QoS::AtLeastOnce,
            false,
            "{}",
        )
        .await;
    assert!(result.is_err(), "cross-tenant publish should fail");

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_payload_transformation() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            r#"
allow publish gps_telemetry if subject.tenant_id == input.tenant_id

redact publish payload.location.gps if subject.tenant_id == input.tenant_id
"#,
        )
        .await?;

    let client = create_mqtt_client(&harness, &tenant_id, "device-transform").await?;
    client
        .client
        .publish(
            format!("{tenant_id}/sensors/temp"),
            QoS::AtLeastOnce,
            false,
            serde_json::to_vec(&json!({
                "payload": {
                    "location": {
                        "gps": [52.52, 13.405]
                    }
                }
            }))?,
        )
        .await?;

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_subscribe_policy_enforcement() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            r#"
allow subscribe gps_telemetry if subject.roles in ["dispatcher"]
"#,
        )
        .await?;

    let client = create_mqtt_client(&harness, &tenant_id, "device-subscribe").await?;
    client
        .client
        .subscribe(format!("{tenant_id}/sensors/#"), QoS::AtLeastOnce)
        .await?;

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "mqtt-e2e")]
async fn test_mqtt_wildcard_subscription_isolation() -> Result<()> {
    let (mut harness, tenant_id) = setup_mqtt_environment().await?;
    harness
        .deploy_test_policy(
            &tenant_id,
            r#"
allow subscribe gps_telemetry if subject.tenant_id == input.tenant_id
"#,
        )
        .await?;

    let client = create_mqtt_client(&harness, &tenant_id, "device-wildcard").await?;
    client
        .client
        .subscribe(format!("{tenant_id}/#"), QoS::AtLeastOnce)
        .await?;

    let denied = client
        .client
        .subscribe("+/sensors/#", QoS::AtLeastOnce)
        .await;
    assert!(denied.is_err(), "cross-tenant wildcard should be denied");

    harness.cleanup().await?;
    Ok(())
}
