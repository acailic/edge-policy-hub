use super::{default_tenant_config, random_tenant_id, TestHarness};
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PROXY_UPSTREAM_PATH: &str = "/api/test-resource";

async fn setup_proxy_environment() -> Result<(TestHarness, MockServer, String)> {
    let mut harness = TestHarness::new().await?;
    let upstream = MockServer::start().await;

    // Configure proxy upstream via env variable override.
    std::env::set_var(
        "EDGE_POLICY_PROXY_UPSTREAM_BASE",
        upstream.uri().to_string(),
    );

    harness.start_all_services().await?;

    let tenant_id = random_tenant_id("tenant-http");
    let mut config = default_tenant_config();
    config
        .as_object_mut()
        .unwrap()
        .entry("upstream")
        .or_insert(json!({}))
        .as_object_mut()
        .unwrap()
        .insert("base_url".into(), json!(upstream.uri()));

    harness
        .create_test_tenant(&tenant_id, &config)
        .await
        .expect("tenant created");

    Ok((harness, upstream, tenant_id))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_http_proxy_policy_enforcement_allow() -> Result<()> {
    let (mut harness, upstream, tenant_id) = setup_proxy_environment().await?;
    let policy = r#"
allow read http_request if
  subject.tenant_id == "tenant-http" and
  resource.region == "EU" and
  environment.country == "DE"
"#;

    harness
        .deploy_test_policy(&tenant_id, policy)
        .await
        .expect("policy deployed");

    Mock::given(method("GET"))
        .and(path(PROXY_UPSTREAM_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "ok": true,
                "tenant": tenant_id,
            })),
        )
        .mount(&upstream)
        .await;

    let proxy_url = format!(
        "http://127.0.0.1:{}{}",
        harness.ports().proxy_http,
        PROXY_UPSTREAM_PATH
    );
    let response = harness
        .http_client()
        .get(proxy_url)
        .header("X-Tenant-ID", &tenant_id)
        .header("X-Region", "EU")
        .header("X-Country", "DE")
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["ok"], json!(true));

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_http_proxy_policy_enforcement_deny() -> Result<()> {
    let (mut harness, upstream, tenant_id) = setup_proxy_environment().await?;
    let policy = r#"
allow read http_request if
  subject.tenant_id == "tenant-http" and
  environment.country == "EU"
"#;
    harness
        .deploy_test_policy(&tenant_id, policy)
        .await
        .expect("policy deployed");

    Mock::given(method("GET"))
        .and(path(PROXY_UPSTREAM_PATH))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&upstream)
        .await;

    let proxy_url = format!(
        "http://127.0.0.1:{}{}",
        harness.ports().proxy_http,
        PROXY_UPSTREAM_PATH
    );
    let response = harness
        .http_client()
        .get(proxy_url)
        .header("X-Tenant-ID", &tenant_id)
        .header("X-Country", "US")
        .send()
        .await?;
    assert_eq!(response.status(), 403);

    // Allow upstream expectations to settle.
    sleep(Duration::from_millis(200)).await;

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_http_proxy_field_redaction() -> Result<()> {
    let (mut harness, upstream, tenant_id) = setup_proxy_environment().await?;
    let policy = r#"
allow read http_request if
  subject.tenant_id == "tenant-http"

redact http_response pii.email if
  subject.tenant_id == "tenant-http"
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "profile": {
                    "name": "alice",
                    "pii.email": "alice@example.com"
                }
            })),
        )
        .mount(&upstream)
        .await;

    let proxy_url = format!("http://127.0.0.1:{}{}", harness.ports().proxy_http, PROXY_UPSTREAM_PATH);
    let response = harness
        .http_client()
        .get(proxy_url)
        .header("X-Tenant-ID", &tenant_id)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["profile"]["pii.email"], serde_json::Value::Null);

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_http_proxy_quota_enforcement() -> Result<()> {
    let (mut harness, upstream, tenant_id) = setup_proxy_environment().await?;
    let policy = r#"
allow read http_request if
  subject.tenant_id == "tenant-http" and
  environment.bandwidth_used < 8
"#;
    harness.deploy_test_policy(&tenant_id, policy).await?;

    Mock::given(method("GET"))
        .and(path(PROXY_UPSTREAM_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 4096]))
        .mount(&upstream)
        .await;

    let proxy_url = format!(
        "http://127.0.0.1:{}{}",
        harness.ports().proxy_http,
        PROXY_UPSTREAM_PATH
    );
    for _ in 0..2 {
        let response = harness
            .http_client()
            .get(&proxy_url)
            .header("X-Tenant-ID", &tenant_id)
            .send()
            .await?;
        assert_eq!(response.status(), 200);
    }

    let denied = harness
        .http_client()
        .get(&proxy_url)
        .header("X-Tenant-ID", &tenant_id)
        .send()
        .await?;
    assert_eq!(denied.status(), 429);

    harness.cleanup().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_http_proxy_cross_tenant_isolation() -> Result<()> {
    let (mut harness, upstream, tenant_a) = setup_proxy_environment().await?;
    let tenant_b = random_tenant_id("tenant-http");

    harness.create_test_tenant(&tenant_b, &default_tenant_config()).await?;

    let policy_a = r#"
allow read http_request if
  subject.tenant_id == "tenant-http"
"#;
    harness.deploy_test_policy(&tenant_a, policy_a).await?;

    let policy_b = r#"
deny read http_request if
  subject.tenant_id == "tenant-http"
"#;
    harness.deploy_test_policy(&tenant_b, policy_b).await?;

    Mock::given(method("GET"))
        .and(path(PROXY_UPSTREAM_PATH))
        .respond_with(ResponseTemplate::new(200))
        .mount(&upstream)
        .await;

    let proxy_url = format!(
        "http://127.0.0.1:{}{}",
        harness.ports().proxy_http,
        PROXY_UPSTREAM_PATH
    );

    let allow = harness
        .http_client()
        .get(&proxy_url)
        .header("X-Tenant-ID", &tenant_a)
        .send()
        .await?;
    assert!(allow.status().is_success());

    let deny = harness
        .http_client()
        .get(&proxy_url)
        .header("X-Tenant-ID", &tenant_a)
        .header("X-Resource-Tenant", &tenant_b)
        .send()
        .await?;
    assert_eq!(deny.status(), 403);

    harness.cleanup().await?;
    Ok(())
}
