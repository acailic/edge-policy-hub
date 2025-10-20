use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use edge_policy_hub::bench_support::{default_tenant_config, random_tenant_id, TestHarness};
use reqwest::Client;
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use rumqttc::{AsyncClient, MqttOptions, QoS};

fn bench_http_proxy_latency(c: &mut Criterion) {
    let runtime = Runtime::new().expect("runtime");
    let mut group = c.benchmark_group("e2e_latency");
    group
        .sample_size(200)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));

    group.bench_function(BenchmarkId::new("http_proxy_e2e", "allow"), |b| {
        let (mut harness, proxy_url, tenant_id) = runtime
            .block_on(async {
                let mut harness = TestHarness::new().await.expect("harness");
                let upstream = MockServer::start().await;
                std::env::set_var(
                    "EDGE_POLICY_PROXY_UPSTREAM_BASE",
                    upstream.uri().to_string(),
                );
                harness.start_all_services().await.expect("services");
                let tenant_id = random_tenant_id("bench-http");
                harness
                    .create_test_tenant(&tenant_id, &default_tenant_config())
                    .await
                    .expect("tenant");
                harness
                    .deploy_test_policy(
                        &tenant_id,
                        &format!(
                            r#"
allow read http_request if
  subject.tenant_id == "{tenant_id}"
"#
                        ),
                    )
                    .await
                    .expect("policy");

                Mock::given(method("GET"))
                    .and(path("/bench"))
                    .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
                    .mount(&upstream)
                    .await;

                let proxy_url = format!(
                    "http://127.0.0.1:{}/bench",
                    harness.ports().proxy_http
                );

                (harness, proxy_url, tenant_id)
            });

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client");

        b.iter(|| {
            runtime
                .block_on(async {
                    let response = client
                        .get(&proxy_url)
                        .header("X-Tenant-ID", &tenant_id)
                        .send()
                        .await
                        .expect("response");
                    assert_eq!(response.status(), 200, "Expected 200 OK for allowed request");
                    response.bytes().await.expect("bytes");
                });
        });

        runtime
            .block_on(async {
                harness.cleanup().await.expect("cleanup");
            });
    });

    group.bench_function(BenchmarkId::new("mqtt_publish_e2e", "allow"), |b| {
        let (mut harness, options, topic, _tenant_id) = runtime.block_on(async {
            let mut harness = TestHarness::new().await.expect("harness");
            harness.start_all_services().await.expect("services");
            let tenant_id = random_tenant_id("bench-mqtt");
            harness
                .create_test_tenant(&tenant_id, &default_tenant_config())
                .await
                .expect("tenant");
            harness
                .deploy_test_policy(
                    &tenant_id,
                    &format!(
                        r#"
allow publish gps_telemetry if subject.tenant_id == "{tenant_id}"
allow subscribe gps_telemetry if subject.tenant_id == "{tenant_id}"
"#
                    ),
                )
                .await
                .expect("policy");

            let mut options = MqttOptions::new(
                format!("{tenant_id}/device-bench"),
                "127.0.0.1",
                harness.ports().mqtt_bridge,
            );
            options.set_keep_alive(Duration::from_secs(5));
            let topic = format!("{tenant_id}/sensors/temp");

            (harness, options, topic, tenant_id)
        });

        b.iter(|| {
            runtime.block_on(async {
                // Create fresh client for each iteration to measure true e2e latency
                let (client, mut event_loop) = AsyncClient::new(options.clone(), 10);

                // Spawn event loop handler
                let ack_receiver = tokio::spawn(async move {
                    while let Ok(notification) = event_loop.poll().await {
                        if let rumqttc::Event::Incoming(rumqttc::Packet::PubAck(_)) = notification {
                            return;
                        }
                    }
                });

                let start = std::time::Instant::now();

                // Publish with QoS1 to get PUBACK
                client
                    .publish(
                        topic.clone(),
                        QoS::AtLeastOnce,
                        false,
                        r#"{"value": 42}"#,
                    )
                    .await
                    .expect("publish");

                // Wait for PUBACK to measure true end-to-end latency
                tokio::time::timeout(Duration::from_secs(5), ack_receiver)
                    .await
                    .expect("timeout waiting for puback")
                    .expect("event loop failed");

                let _elapsed = start.elapsed();
            });
        });

        runtime.block_on(async {
            harness.cleanup().await.expect("cleanup");
        });
    });

    group.finish();
}

criterion_group!(end_to_end_latency, bench_http_proxy_latency);
criterion_main!(end_to_end_latency);
