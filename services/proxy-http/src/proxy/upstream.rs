use super::ProxyError;
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Response, Uri};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use reqwest::Client;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info, instrument};

pub struct UpstreamClient {
    http_client: Client,
    upstream_base_url: String,
    max_body_size_bytes: usize,
}

impl UpstreamClient {
    pub fn new(upstream_url: String, timeout_secs: u64, max_body_size_bytes: usize) -> anyhow::Result<Self> {
        // Build client with both HTTP/1.1 and HTTP/2 support
        // Protocol negotiation via ALPN or upgrade
        let http_client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(20)
            // Removed .http2_prior_knowledge() to support both HTTP/1.1 and HTTP/2
            .build()?;

        Ok(Self {
            http_client,
            upstream_base_url: upstream_url.trim_end_matches('/').to_string(),
            max_body_size_bytes,
        })
    }

    #[instrument(skip(self, req), fields(method = %req.method(), path = %req.uri().path()))]
    pub async fn forward_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, ProxyError> {
        let (parts, body) = req.into_parts();

        // Build upstream URL
        let path_and_query = parts.uri.path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        let upstream_url = format!("{}{}", self.upstream_base_url, path_and_query);

        debug!(upstream_url = %upstream_url, "Forwarding request to upstream");

        // Collect request body
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| ProxyError::Upstream(format!("Failed to read request body: {}", e)))?
            .to_bytes();

        // Check body size limit
        if body_bytes.len() > self.max_body_size_bytes {
            return Err(ProxyError::BodyTooLarge {
                size: body_bytes.len(),
                limit: self.max_body_size_bytes,
            });
        }

        // Sanitize headers
        let headers = Self::sanitize_headers(&parts.headers);

        // Build upstream request
        let mut upstream_req = self
            .http_client
            .request(parts.method.clone(), &upstream_url);

        // Add headers
        for (name, value) in headers.iter() {
            upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
        }

        // Add forwarded headers
        upstream_req = upstream_req
            .header("X-Forwarded-Proto", "http")
            .header("X-Forwarded-Host", parts.uri.host().unwrap_or("unknown"));

        // Add body if present
        if !body_bytes.is_empty() {
            upstream_req = upstream_req.body(body_bytes.to_vec());
        }

        let start = std::time::Instant::now();

        // Send request
        let upstream_response = upstream_req
            .send()
            .await
            .map_err(|e| ProxyError::Upstream(format!("Upstream request failed: {}", e)))?;

        let latency = start.elapsed();
        let status = upstream_response.status();

        info!(
            status = status.as_u16(),
            latency_ms = latency.as_millis(),
            "Upstream response received"
        );

        // Convert reqwest::Response to hyper::Response
        let mut response_builder = Response::builder().status(status);

        // Copy headers
        for (name, value) in upstream_response.headers().iter() {
            response_builder = response_builder.header(name, value);
        }

        // Get body
        let response_body = upstream_response
            .bytes()
            .await
            .map_err(|e| ProxyError::Upstream(format!("Failed to read upstream response: {}", e)))?;

        let response = response_builder
            .body(Full::new(response_body))
            .map_err(|e| ProxyError::Upstream(format!("Failed to build response: {}", e)))?;

        Ok(response)
    }

    fn sanitize_headers(headers: &HeaderMap) -> HeaderMap {
        let mut sanitized = HeaderMap::new();

        // List of hop-by-hop headers to remove
        const HOP_BY_HOP: &[&str] = &[
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailers",
            "transfer-encoding",
            "upgrade",
        ];

        // List of internal headers to remove
        const INTERNAL: &[&str] = &[
            "x-tenant-id",
            "authorization", // Will be re-added if needed by upstream
        ];

        for (name, value) in headers.iter() {
            let name_lower = name.as_str().to_lowercase();

            if HOP_BY_HOP.contains(&name_lower.as_str()) {
                continue;
            }

            if INTERNAL.contains(&name_lower.as_str()) {
                continue;
            }

            sanitized.insert(name.clone(), value.clone());
        }

        sanitized
    }
}
