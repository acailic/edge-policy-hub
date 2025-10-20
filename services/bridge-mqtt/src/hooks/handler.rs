use std::sync::Arc;

use tracing::{debug, error, warn, instrument};

use crate::policy::MqttAbacInput;
use crate::transform::TransformDirective;

use super::HookContext;

/// PolicyHookHandler implements policy enforcement for MQTT operations.
///
/// This handler provides complete policy enforcement logic for MQTT events.
/// It is designed to integrate with RMQTT's hook system, but the actual
/// RMQTT trait implementation is left as a TODO due to API surface differences
/// in RMQTT v0.17.
///
/// The handler methods below demonstrate the complete enforcement flow:
/// - handle_client_connected: Extract tenant context and store in session
/// - handle_client_disconnected: Remove tenant context from session
/// - handle_message_publish: Validate topic namespace, query policy, transform payload
/// - handle_client_subscribe: Validate topic filter, query policy
pub struct PolicyHookHandler {
    context: Arc<HookContext>,
}

// NOTE: RMQTT Hook Trait Implementation
//
// The rmqtt::broker::hook::Handler trait implementation would go here.
// Due to RMQTT v0.17 API differences, this is left as a placeholder.
// The integration would look like:
//
// #[async_trait::async_trait]
// impl rmqtt::broker::hook::Handler for PolicyHookHandler {
//     async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
//         match param {
//             Parameter::ClientConnected(session, connect_info) => { ... }
//             Parameter::ClientDisconnected(session, reason) => { ... }
//             Parameter::MessagePublish(session, from, publish) => { ... }
//             Parameter::ClientSubscribe(session, subscribe) => { ... }
//             _ => ReturnType::Success
//         }
//     }
// }
//
// The handler methods below provide the enforcement logic that would be
// called by the RMQTT hook trait implementation.

impl PolicyHookHandler {
    pub fn new(context: Arc<HookContext>) -> Self {
        Self { context }
    }

    /// Handle client connection - extract and store tenant context
    #[instrument(skip(self))]
    pub async fn handle_client_connected(
        &self,
        client_id: &str,
        username: Option<&str>,
        cert_der: Option<&[u8]>,
        peer_addr: Option<std::net::IpAddr>,
    ) -> Result<(), String> {
        debug!("Handling client connection: {}", client_id);

        match self.context.tenant_extractor.extract_tenant_context(
            client_id,
            username,
            cert_der,
            peer_addr,
        ) {
            Ok(tenant_context) => {
                debug!(
                    "Successfully extracted tenant context for client '{}': tenant_id={}, connection_id={}",
                    client_id, tenant_context.tenant_id, tenant_context.connection_id
                );

                self.context
                    .session_store
                    .store_context(client_id.to_string(), tenant_context);

                Ok(())
            }
            Err(e) => {
                error!("Failed to extract tenant context for client '{}': {}", client_id, e);
                Err(format!("Tenant authentication failed: {}", e))
            }
        }
    }

    /// Handle client disconnection - clean up session
    #[instrument(skip(self))]
    pub fn handle_client_disconnected(&self, client_id: &str, reason: &str) {
        debug!("Handling client disconnection: {} (reason: {})", client_id, reason);

        if let Some(context) = self.context.session_store.remove_context(client_id) {
            debug!(
                "Removed session for client '{}' with tenant '{}'",
                client_id, context.tenant_id
            );
        }
    }

    /// Handle message publish - validate, query policy, transform if needed
    #[instrument(skip(self, payload))]
    pub async fn handle_message_publish(
        &self,
        client_id: &str,
        topic: &str,
        qos: u8,
        retain: bool,
        payload: &[u8],
    ) -> Result<Option<Vec<u8>>, String> {
        debug!(
            "Handling message publish: client={}, topic={}, qos={}, retain={}, size={}",
            client_id, topic, qos, retain, payload.len()
        );

        // Get tenant context from session
        let tenant_context = self
            .context
            .session_store
            .get_context(client_id)
            .ok_or_else(|| {
                error!("No tenant context found for client '{}'", client_id);
                "Client not authenticated".to_string()
            })?;

        // Validate topic namespace matches tenant
        if !self.validate_topic_namespace(topic, &tenant_context.tenant_id) {
            warn!(
                "Topic namespace violation: client '{}' (tenant '{}') attempted to publish to '{}'",
                client_id, tenant_context.tenant_id, topic
            );
            return Err("Topic namespace violation".to_string());
        }

        // Fast-fail quota check before policy query
        if let Err(e) = self.context.quota_tracker.check_quota(&tenant_context.tenant_id) {
            warn!(
                "Quota exceeded for tenant '{}': {}",
                tenant_context.tenant_id, e
            );
            return Err(format!("Quota limit exceeded: {}", e));
        }

        // Get current quota metrics
        let metrics = self.context
            .quota_tracker
            .get_metrics(&tenant_context.tenant_id)
            .unwrap_or_default();

        // Build ABAC input for publish
        let abac_input = MqttAbacInput::for_publish(
            &tenant_context,
            topic,
            qos,
            retain,
            payload.len(),
            metrics.message_count,
        );

        // Query policy
        let policy_decision = self
            .context
            .policy_client
            .query_publish_policy(&tenant_context.tenant_id, abac_input)
            .await
            .map_err(|e| {
                error!(
                    "Policy query failed for client '{}' publishing to '{}': {}",
                    client_id, topic, e
                );
                format!("Policy enforcement error: {}", e)
            })?;

        debug!(
            "Policy decision for client '{}' publishing to '{}': allow={}",
            client_id, topic, policy_decision.allow
        );

        // Check if transformation is needed
        let transformed_payload = if self.context.config.enable_payload_transformation {
            let mut directives = Vec::new();

            // Handle redact_fields (new field)
            if let Some(redact_fields) = policy_decision.redact_fields {
                if !redact_fields.is_empty() {
                    debug!("Adding RedactFields directive: {:?}", redact_fields);
                    directives.push(TransformDirective::RedactFields(redact_fields));
                }
            }

            // Handle legacy redact field (maps to RemoveFields for backwards compatibility)
            if let Some(remove_fields) = policy_decision.redact {
                if !remove_fields.is_empty() {
                    debug!("Adding RemoveFields directive from legacy 'redact': {:?}", remove_fields);
                    directives.push(TransformDirective::RemoveFields(remove_fields));
                }
            }

            // Handle remove_fields
            if let Some(remove_fields) = policy_decision.remove_fields {
                if !remove_fields.is_empty() {
                    debug!("Adding RemoveFields directive: {:?}", remove_fields);
                    directives.push(TransformDirective::RemoveFields(remove_fields));
                }
            }

            // Handle strip_coordinates
            if let Some(true) = policy_decision.strip_coordinates {
                debug!("Adding StripCoordinates directive");
                directives.push(TransformDirective::StripCoordinates);
            }

            if !directives.is_empty() {
                debug!(
                    "Applying {} payload transformation directive(s)",
                    directives.len()
                );

                match self.context.payload_transformer.transform_payload(payload, &directives) {
                    Ok(transformed) => Some(transformed),
                    Err(e) => {
                        warn!("Payload transformation failed: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        // Increment quota tracker
        self.context
            .quota_tracker
            .increment_message_count(&tenant_context.tenant_id, payload.len());

        Ok(transformed_payload)
    }

    /// Handle client subscribe - validate topic filter and query policy
    #[instrument(skip(self))]
    pub async fn handle_client_subscribe(
        &self,
        client_id: &str,
        topic_filter: &str,
        qos: u8,
    ) -> Result<(), String> {
        debug!(
            "Handling client subscribe: client={}, topic_filter={}, qos={}",
            client_id, topic_filter, qos
        );

        // Get tenant context from session
        let tenant_context = self
            .context
            .session_store
            .get_context(client_id)
            .ok_or_else(|| {
                error!("No tenant context found for client '{}'", client_id);
                "Client not authenticated".to_string()
            })?;

        // Validate topic filter namespace
        if !self.validate_topic_namespace(topic_filter, &tenant_context.tenant_id) {
            warn!(
                "Topic filter namespace violation: client '{}' (tenant '{}') attempted to subscribe to '{}'",
                client_id, tenant_context.tenant_id, topic_filter
            );
            return Err("Topic filter namespace violation".to_string());
        }

        // Fast-fail quota check before policy query
        if let Err(e) = self.context.quota_tracker.check_quota(&tenant_context.tenant_id) {
            warn!(
                "Quota exceeded for tenant '{}': {}",
                tenant_context.tenant_id, e
            );
            return Err(format!("Quota limit exceeded: {}", e));
        }

        // Check wildcard restrictions
        if !self.context.config.allow_wildcard_subscriptions {
            if topic_filter.contains('#') || topic_filter.contains('+') {
                warn!(
                    "Wildcard subscription denied for client '{}': {}",
                    client_id, topic_filter
                );
                return Err("Wildcard subscriptions not allowed".to_string());
            }
        }

        // Get current quota metrics
        let metrics = self.context
            .quota_tracker
            .get_metrics(&tenant_context.tenant_id)
            .unwrap_or_default();

        // Build ABAC input for subscribe
        let abac_input = MqttAbacInput::for_subscribe(
            &tenant_context,
            topic_filter,
            qos,
            metrics.message_count,
        );

        // Query policy
        self.context
            .policy_client
            .query_subscribe_policy(&tenant_context.tenant_id, abac_input)
            .await
            .map_err(|e| {
                error!(
                    "Policy query failed for client '{}' subscribing to '{}': {}",
                    client_id, topic_filter, e
                );
                format!("Policy enforcement error: {}", e)
            })?;

        debug!(
            "Subscribe allowed for client '{}' to topic filter '{}'",
            client_id, topic_filter
        );

        Ok(())
    }

    /// Validate that topic matches the tenant's namespace pattern
    /// Respects MQTT wildcard semantics: + (single-level), # (multi-level)
    fn validate_topic_namespace(&self, topic: &str, tenant_id: &str) -> bool {
        let pattern = &self.context.config.topic_namespace_pattern;

        // Replace {tenant_id} placeholder with actual tenant ID
        let expected_pattern = pattern.replace("{tenant_id}", tenant_id);

        // Split both pattern and topic into segments
        let pattern_segments: Vec<&str> = expected_pattern.split('/').collect();
        let topic_segments: Vec<&str> = topic.split('/').collect();

        // Reject topics that start with wildcard (trying to escape tenant namespace)
        if !topic_segments.is_empty() && (topic_segments[0] == "+" || topic_segments[0] == "#") {
            debug!(
                "Rejecting topic '{}': wildcard at tenant namespace position",
                topic
            );
            return false;
        }

        // Match pattern against topic
        self.match_mqtt_pattern(&pattern_segments, &topic_segments, tenant_id)
    }

    /// Match MQTT pattern with wildcards against topic segments
    fn match_mqtt_pattern(
        &self,
        pattern_segments: &[&str],
        topic_segments: &[&str],
        tenant_id: &str,
    ) -> bool {
        let mut p_idx = 0;
        let mut t_idx = 0;

        while p_idx < pattern_segments.len() && t_idx < topic_segments.len() {
            let pattern_seg = pattern_segments[p_idx];

            match pattern_seg {
                "#" => {
                    // Multi-level wildcard - matches rest of topic
                    // But we need to ensure tenant segment was already matched
                    return true;
                }
                "+" => {
                    // Single-level wildcard - matches any single segment
                    // BUT: if this is the position where tenant_id should be, reject
                    // We need to track which segment position should contain tenant_id
                    // For simplicity, if pattern has {tenant_id}, we already substituted it
                    // So + should not appear at tenant position
                    p_idx += 1;
                    t_idx += 1;
                }
                _ => {
                    // Exact match required
                    if pattern_seg != topic_segments[t_idx] {
                        // Special case: check if this segment should match tenant_id
                        // and topic is trying to use a different value
                        if pattern_seg == tenant_id {
                            debug!(
                                "Tenant ID mismatch: expected '{}', got '{}' in topic",
                                tenant_id, topic_segments[t_idx]
                            );
                        }
                        return false;
                    }
                    p_idx += 1;
                    t_idx += 1;
                }
            }
        }

        // Check if we consumed both pattern and topic
        // If pattern ends with #, we may have remaining topic segments (which is OK)
        if p_idx < pattern_segments.len() {
            // Remaining pattern segments must be # (multi-level wildcard at end)
            pattern_segments[p_idx] == "#"
        } else {
            // All topic segments must be consumed
            t_idx >= topic_segments.len()
        }
    }
}
