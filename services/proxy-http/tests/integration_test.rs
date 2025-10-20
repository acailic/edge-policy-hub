use std::net::TcpListener;
use std::time::Duration;

use anyhow::Result;
use edge_policy_proxy_http::config::{JwtAlgorithm, ProxyConfig};
use edge_policy_proxy_http::server::ProxyServer;
use reqwest::Client;
use serde_json::json;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TENANT_HEADER: &str = "X-Tenant-ID";

fn unused_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind ephemeral port")
        .local_addr()
        .expect("listener has no local addr")
        .port()
}

fn base_config(enforcer_url: String, upstream_url: String, port: u16) -> ProxyConfig {
    ProxyConfig {
        host: "127.0.0.1".to_string(),
        port,
        upstream_url,
        request_timeout_secs: 2,
        max_body_size_bytes: 1024 * 1024,
        enforcer_url,
        enable_mtls: false,
        tls_cert_path: None,
        tls_key_path: None,
        tls_client_ca_path: None,
        enable_jwt: false,
        jwt_secret: None,
        jwt_public_key_path: None,
        jwt_issuer: None,
        jwt_audience: None,
        jwt_algorithm: JwtAlgorithm::RS256,
        forward_auth_header: false,
        log_level: "warn".to_string(),
        quota_tracker_url: None,
        quota_tracker_token: None,
        default_region: None,
    }
}

async fn start_proxy(config: ProxyConfig) -> (JoinHandle<Result<()>>, String) {
    let addr = format!("{}:{}", config.host, config.port);
    let base_url = format!("http://{}", addr);
    config.validate().expect("config validation failed");
    let server = ProxyServer::new(config).expect("failed to construct proxy server");
    let handle = tokio::spawn(async move { server.run().await });
    wait_for_port(&addr).await;
    (handle, base_url)
}

async fn wait_for_port(addr: &str) {
    for _ in 0..10 {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(_) => return,
            Err(_) => sleep(Duration::from_millis(50)).await,
        }
    }
    panic!("proxy [{}] did not become ready in time", addr);
}

async fn teardown(handle: JoinHandle<Result<()>>) {
    handle.abort();
    let _ = handle.await;
}

fn tenant_header_value() -> &'static str {
    "tenant-integration"
}

#[tokio::test(flavor = "multi_thread")]
async fn allow_requests_reach_upstream() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": { "allow": true }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "ok"
        })))
        .mount(&upstream)
        .await;

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer.uri(), upstream.uri(), port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    let response = client
        .get(format!("{}/data", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body, json!({ "status": "ok" }));

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_requests_do_not_hit_upstream() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": { "allow": false, "reason": "blocked" }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&upstream)
        .await;

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer.uri(), upstream.uri(), port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client
        .get(format!("{}/data", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 403);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["error"], json!("POLICY_DENIED"));

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn redaction_is_applied_to_json_responses() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": {
                "allow": true,
                "redact": ["pii.email"]
            }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/profile"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "Alice",
            "pii": {
                "email": "alice@example.com",
                "phone": "+15551234567"
            }
        })))
        .mount(&upstream)
        .await;

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer.uri(), upstream.uri(), port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client
        .get(format!("{}/profile", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(
        payload,
        json!({
            "name": "Alice",
            "pii": { "phone": "+15551234567" }
        })
    );

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn non_json_responses_are_not_redacted() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": {
                "allow": true,
                "redact": ["pii.email"]
            }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/binary"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/octet-stream")
                .set_body_bytes(vec![1, 2, 3, 4]),
        )
        .mount(&upstream)
        .await;

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer.uri(), upstream.uri(), port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let bytes = client
        .get(format!("{}/binary", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?
        .bytes()
        .await?;

    assert_eq!(bytes.as_ref(), &[1, 2, 3, 4]);

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn enforcer_unavailable_yields_service_unavailable() -> Result<()> {
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
        .mount(&upstream)
        .await;

    let enforcer_port = unused_port();
    let enforcer_url = format!("http://127.0.0.1:{}", enforcer_port);

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer_url, upstream.uri(), port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client
        .get(format!("{}/data", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 503);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["error"], json!("ENFORCER_UNREACHABLE"));

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn upstream_unavailable_returns_bad_gateway() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": { "allow": true }
        })))
        .mount(&enforcer)
        .await;

    let upstream_port = unused_port();
    let upstream_url = format!("http://127.0.0.1:{}", upstream_port);

    let port = unused_port();
    let (handle, base_url) = start_proxy(base_config(enforcer.uri(), upstream_url, port)).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client
        .get(format!("{}/data", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 502);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["error"], json!("UPSTREAM_ERROR"));

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn upstream_timeout_results_in_gateway_timeout() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": { "allow": true }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "delayed": true }))
                .set_delay(Duration::from_secs(3)),
        )
        .mount(&upstream)
        .await;

    let port = unused_port();
    let mut config = base_config(enforcer.uri(), upstream.uri(), port);
    config.request_timeout_secs = 1;
    let (handle, base_url) = start_proxy(config).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client
        .get(format!("{}/slow", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .send()
        .await?;

    assert_eq!(response.status(), 504);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["error"], json!("TIMEOUT"));

    teardown(handle).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_body_over_limit_is_rejected() -> Result<()> {
    let enforcer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/data/tenants/tenant-integration/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": { "allow": true }
        })))
        .mount(&enforcer)
        .await;

    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&upstream)
        .await;

    let port = unused_port();
    let mut config = base_config(enforcer.uri(), upstream.uri(), port);
    config.max_body_size_bytes = 16;
    let (handle, base_url) = start_proxy(config).await;

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let body = vec![b'a'; 32];
    let response = client
        .post(format!("{}/upload", base_url))
        .header(TENANT_HEADER, tenant_header_value())
        .body(body)
        .send()
        .await?;

    assert_eq!(response.status(), 413);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["error"], json!("BODY_TOO_LARGE"));

    teardown(handle).await;
    Ok(())
}
