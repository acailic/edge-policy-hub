use super::{ProxyError, ProxyState};
use crate::config::ProxyConfig;
use crate::policy::AbacInput;
use crate::server::PeerInfo;
use bytes::Bytes;
use http::{HeaderValue, Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Incoming};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

pub struct ProxyHandler {
    state: ProxyState,
}

impl ProxyHandler {
    pub fn new(config: Arc<ProxyConfig>) -> anyhow::Result<Self> {
        let state = ProxyState::new((*config).clone())?;
        Ok(Self { state })
    }

    #[instrument(skip(self, req), fields(request_id))]
    pub async fn handle_request(
        &self,
        req: Request<Incoming>,
        peer_info: Option<Arc<PeerInfo>>,
    ) -> Result<Response<Full<Bytes>>, ProxyError> {
        // Wrap entire pipeline in timeout
        let timeout_duration = self.state.config.request_timeout();

        match tokio::time::timeout(timeout_duration, self.handle_request_inner(req, peer_info))
            .await
        {
            Ok(result) => result,
            Err(_) => Err(ProxyError::Timeout),
        }
    }

    async fn handle_request_inner(
        &self,
        req: Request<Incoming>,
        peer_info: Option<Arc<PeerInfo>>,
    ) -> Result<Response<Full<Bytes>>, ProxyError> {
        let start = std::time::Instant::now();

        // Extract peer certificates and client IP from peer_info
        let peer_certs = peer_info.as_ref().map(|info| {
            info.certificates
                .iter()
                .map(|cert| cert.as_ref().to_vec())
                .collect::<Vec<Vec<u8>>>()
        });

        let client_ip = peer_info.as_ref().map(|info| info.addr.ip());

        // Step 1: Extract tenant context
        debug!("Step 1: Extracting tenant context");
        let mut tenant_context = self
            .state
            .tenant_extractor
            .extract_from_request(req.headers(), peer_certs.as_deref())?;

        // Set client IP if available
        if let Some(ip) = client_ip {
            tenant_context.client_ip = Some(ip);
        }

        let quota_usage_bytes = if let Some(quota_client) = &self.state.quota_client {
            match quota_client.get_usage(&tenant_context.tenant_id).await {
                Ok(usage) => Some(usage.bandwidth_bytes),
                Err(err) => {
                    warn!(
                        tenant_id = %tenant_context.tenant_id,
                        error = %err,
                        "Failed to fetch quota usage; proceeding without bandwidth context"
                    );
                    None
                }
            }
        } else {
            None
        };

        let request_id = tenant_context.request_id.clone();
        tracing::Span::current().record("request_id", &request_id);

        info!(
            tenant_id = %tenant_context.tenant_id,
            user_id = ?tenant_context.user_id,
            auth_method = ?tenant_context.auth_method,
            "Request authenticated"
        );

        // Step 2: Build ABAC input
        debug!("Step 2: Building ABAC input");
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|q| q.to_string());

        let mut abac_input = AbacInput::from_request(
            &tenant_context,
            &method,
            &path,
            query.as_deref(),
            req.headers(),
        );

        if let Some(bytes) = quota_usage_bytes {
            abac_input.environment.bandwidth_used = Some(bytes as f64);
        }

        debug!(abac_input = ?abac_input, "ABAC input prepared");

        // Step 3: Query policy enforcer
        debug!("Step 3: Querying policy enforcer");
        let policy_start = std::time::Instant::now();
        let policy_decision = self
            .state
            .policy_client
            .query_policy(&tenant_context.tenant_id, abac_input)
            .await?;
        let policy_latency = policy_start.elapsed();

        info!(
            tenant_id = %tenant_context.tenant_id,
            allow = policy_decision.allow,
            redact_paths = ?policy_decision.redact,
            policy_latency_ms = policy_latency.as_millis(),
            "Policy decision received"
        );

        // Step 4: Forward request to upstream
        debug!("Step 4: Forwarding request to upstream");
        let upstream_start = std::time::Instant::now();
        let forwarded = self.state.upstream_client.forward_request(req).await?;
        let upstream_latency = upstream_start.elapsed();
        let mut upstream_response = forwarded.response;
        let request_body_bytes = forwarded.request_body_bytes;
        let mut response_body_bytes = forwarded.response_body_bytes;

        debug!(
            status = upstream_response.status().as_u16(),
            upstream_latency_ms = upstream_latency.as_millis(),
            "Upstream response received"
        );

        // Step 5: Apply redaction if needed
        if let Some(redact_paths) = &policy_decision.redact {
            if !redact_paths.is_empty() {
                debug!("Step 5: Applying redaction");

                // Check if response is JSON
                let content_type = upstream_response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                if content_type.contains("application/json") {
                    // Extract body
                    let (parts, body) = upstream_response.into_parts();
                    let body_bytes = body
                        .collect()
                        .await
                        .map_err(|e| {
                            ProxyError::Upstream(format!("Failed to read response body: {}", e))
                        })?
                        .to_bytes();

                    // Check body size limit
                    let body_len = body_bytes.len();
                    if body_len > self.state.config.max_body_size_bytes {
                        warn!(
                            size = body_len,
                            limit = self.state.config.max_body_size_bytes,
                            "Response body exceeds max size, skipping redaction"
                        );
                        response_body_bytes = body_len;
                        upstream_response = Response::from_parts(parts, Full::new(body_bytes));
                    } else {
                        // Apply redaction
                        match self
                            .state
                            .redaction_engine
                            .redact_fields(&body_bytes, redact_paths)
                        {
                            Ok(redacted_bytes) => {
                                let redacted = Bytes::from(redacted_bytes);
                                let redacted_len = redacted.len();
                                info!(
                                    original_size = body_len,
                                    redacted_size = redacted_len,
                                    paths = ?redact_paths,
                                    "Redaction applied"
                                );

                                // Rebuild response with redacted body
                                let mut response = Response::from_parts(parts, Full::new(redacted));

                                // Update Content-Length header
                                let body_len = response.body().size_hint().exact().unwrap_or(0);
                                response.headers_mut().insert(
                                    "content-length",
                                    HeaderValue::from_str(&body_len.to_string()).unwrap(),
                                );

                                response_body_bytes = redacted_len;
                                upstream_response = response;
                            }
                            Err(e) => {
                                error!(error = %e, "Redaction failed, returning original response");
                                // Rebuild response with original body
                                response_body_bytes = body_len;
                                upstream_response =
                                    Response::from_parts(parts, Full::new(body_bytes));
                            }
                        }
                    }
                } else {
                    debug!(
                        content_type = content_type,
                        "Response is not JSON, skipping redaction"
                    );
                }
            }
        }

        // Step 6: Log bandwidth usage (for future quota integration)
        let response_size = upstream_response
            .body()
            .size_hint()
            .exact()
            .unwrap_or(response_body_bytes as u64);

        debug!(
            response_size_bytes = response_size,
            request_size_bytes = request_body_bytes,
            "Computed bandwidth usage for quota tracking"
        );
        if let Some(quota_client) = &self.state.quota_client {
            let total_bytes = (request_body_bytes as u64).saturating_add(response_size);
            if let Err(err) = quota_client
                .increment(&tenant_context.tenant_id, total_bytes, &request_id)
                .await
            {
                warn!(
                    tenant_id = %tenant_context.tenant_id,
                    error = %err,
                    "Failed to update quota usage after request"
                );
            }
        }

        // Step 7: Audit logging
        let total_latency = start.elapsed();
        info!(
            tenant_id = %tenant_context.tenant_id,
            user_id = ?tenant_context.user_id,
            request_id = %request_id,
            method = %method,
            path = %path,
            status = upstream_response.status().as_u16(),
            policy_decision = "allow",
            redaction_applied = policy_decision.redact.is_some(),
            policy_latency_ms = policy_latency.as_millis(),
            upstream_latency_ms = upstream_latency.as_millis(),
            total_latency_ms = total_latency.as_millis(),
            response_size_bytes = response_size,
            "Request completed"
        );

        // Add request ID to response headers
        let (mut parts, body) = upstream_response.into_parts();
        parts
            .headers
            .insert("x-request-id", HeaderValue::from_str(&request_id).unwrap());

        Ok(Response::from_parts(parts, body))
    }
}
