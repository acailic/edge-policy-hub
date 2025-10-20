use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn};

// RMQTT imports
use rmqtt::context::ServerContext;
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType, Type};
use rmqtt::net::Builder;
use rmqtt::server::MqttServer;
use rmqtt::types::{PublishAclResult, SubscribeAclResult};
use rmqtt::session::Session;
use rmqtt::codec::v3;

use crate::config::BridgeConfig;
use crate::hooks::{HookContext, PolicyHookHandler};

pub struct MqttBroker {
    config: Arc<BridgeConfig>,
    hook_context: Arc<HookContext>,
}

impl MqttBroker {
    pub fn new(config: BridgeConfig) -> Result<Self> {
        let hook_context = Arc::new(HookContext::new(config.clone())?);

        Ok(Self {
            config: Arc::new(config),
            hook_context,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting MQTT broker on {}:{}",
            self.config.broker_host, self.config.broker_port
        );
        info!("Broker name: {}", self.config.broker_name);
        info!("TLS enabled: {}", self.config.enable_tls);
        info!("mTLS enabled: {}", self.config.enable_mtls);
        info!("Topic namespace pattern: {}", self.config.topic_namespace_pattern);
        info!("Enforcer URL: {}", self.config.enforcer_url);

        // Create server context
        let scx = ServerContext::new()
            .node_id(1)
            .build()
            .await;

        // Register policy hook handler
        info!("Registering policy enforcement hooks");
        let register = scx.extends.hook_mgr().register();
        let policy_handler = RmqttPolicyAdapter::new(self.hook_context.clone());

        // Register hooks for all relevant events
        register.add_priority(Type::ClientConnected, 0, Box::new(policy_handler.clone())).await;
        register.add_priority(Type::ClientDisconnected, 0, Box::new(policy_handler.clone())).await;
        register.add_priority(Type::MessagePublishCheckAcl, 0, Box::new(policy_handler.clone())).await;
        register.add_priority(Type::MessagePublish, 0, Box::new(policy_handler.clone())).await;
        register.add_priority(Type::ClientSubscribeCheckAcl, 0, Box::new(policy_handler)).await;

        // Start the hook handlers
        register.start().await;
        info!("Policy hooks registered and started successfully");

        // Build and configure the MQTT server
        let mut server_builder = MqttServer::new(scx);

        // Configure listener
        info!("Configuring MQTT listener on {}:{}", self.config.broker_host, self.config.broker_port);

        let bind_addr: std::net::IpAddr = self.config.broker_host.parse()
            .map_err(|e| anyhow::anyhow!("Invalid broker host: {}", e))?;

        let mut builder = Builder::new()
            .name(&self.config.broker_name)
            .laddr((bind_addr, self.config.broker_port).into())
            .max_packet_size(self.config.max_payload_size_bytes as u32);

        // Configure TLS if enabled
        if self.config.enable_tls {
            info!("TLS enabled");

            let tls_cert = self.config.tls_cert_path.as_ref()
                .ok_or_else(|| anyhow::anyhow!("TLS cert path required"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid TLS cert path"))?
                .to_string();

            let tls_key = self.config.tls_key_path.as_ref()
                .ok_or_else(|| anyhow::anyhow!("TLS key path required"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid TLS key path"))?
                .to_string();

            builder = builder
                .tls_cert(Some(tls_cert))
                .tls_key(Some(tls_key));

            // Configure mTLS if enabled
            if self.config.enable_mtls {
                info!("mTLS enabled with client certificate validation");

                builder = builder
                    .tls_cross_certificate(true)
                    .cert_cn_as_username(self.config.cert_cn_as_username);
            }
        }

        // Bind and create listener
        let listener = builder.bind()?.tcp()?;
        server_builder = server_builder.listener(listener);

        info!("Listener configured on {}:{}", self.config.broker_host, self.config.broker_port);

        let server = server_builder.build();

        info!("MQTT broker configured successfully, starting server...");

        // Run the server in a background task
        let server_handle = server.clone();
        tokio::spawn(async move {
            if let Err(e) = server_handle.run().await {
                tracing::error!("MQTT server error: {:?}", e);
            }
        });

        info!("MQTT broker started successfully");

        // Wait for shutdown signal
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received SIGINT, shutting down gracefully");
            }
            _ = async {
                #[cfg(unix)]
                {
                    use tokio::signal::unix::{signal, SignalKind};
                    let mut sigterm = signal(SignalKind::terminate())
                        .expect("Failed to create SIGTERM handler");
                    sigterm.recv().await
                }
                #[cfg(not(unix))]
                {
                    std::future::pending::<()>().await
                }
            } => {
                info!("Received SIGTERM, shutting down gracefully");
            }
        }

        info!("MQTT broker shutdown complete");
        Ok(())
    }
}

/// Adapter that bridges our PolicyHookHandler to RMQTT's Handler trait
#[derive(Clone)]
struct RmqttPolicyAdapter {
    context: Arc<HookContext>,
    handler: Arc<PolicyHookHandler>,
}

impl RmqttPolicyAdapter {
    fn new(context: Arc<HookContext>) -> Self {
        let handler = Arc::new(PolicyHookHandler::new(context.clone()));
        Self { context, handler }
    }

    /// Extract client certificate DER from session if available
    fn extract_cert_der(_session: &Session) -> Option<Vec<u8>> {
        // RMQTT stores certificate info in session
        // This is a simplified extraction - actual implementation depends on RMQTT version
        None // TODO: Extract from session.cert_info() or similar
    }

    /// Extract peer IP address from session
    fn extract_peer_addr(session: &Session) -> Option<std::net::IpAddr> {
        session.id.remote_addr.map(|addr| addr.ip())
    }
}

#[async_trait]
impl Handler for RmqttPolicyAdapter {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            // Handle client connection - extract tenant context
            Parameter::ClientConnected(session) => {
                debug!("ClientConnected hook fired for: {:?}", session.id.client_id);

                let client_id = session.id.client_id.as_ref();
                let username = session.id.username.as_ref().map(|s| s.as_ref());
                let cert_der = Self::extract_cert_der(session);
                let peer_addr = Self::extract_peer_addr(session);

                match self.handler.handle_client_connected(
                    client_id,
                    username,
                    cert_der.as_deref(),
                    peer_addr,
                ).await {
                    Ok(_) => {
                        info!("Client connected and authenticated: {}", client_id);
                        (true, acc)
                    }
                    Err(e) => {
                        warn!("Client connection rejected: {} - {}", client_id, e);
                        // Return false to stop processing and reject connection
                        (false, acc)
                    }
                }
            }

            // Handle client disconnection - cleanup session
            Parameter::ClientDisconnected(session, reason) => {
                debug!("ClientDisconnected hook fired for: {:?}", session.id.client_id);

                let client_id = session.id.client_id.as_ref();
                let reason_str = format!("{:?}", reason);

                self.handler.handle_client_disconnected(client_id, &reason_str);

                info!("Client disconnected: {} (reason: {})", client_id, reason_str);
                (true, acc)
            }

            // Handle publish ACL check
            Parameter::MessagePublishCheckAcl(session, publish) => {
                debug!("MessagePublishCheckAcl hook fired for: {:?} topic: {}",
                    session.id.client_id, publish.topic);

                let client_id = session.id.client_id.as_ref();
                let topic: &str = &publish.topic;

                // Quick validation - full check happens in MessagePublish
                // Here we just do a fast namespace check
                let tenant_context = match self.context.session_store.get_context(client_id) {
                    Some(ctx) => ctx,
                    None => {
                        warn!("No tenant context for publish ACL check: {}", client_id);
                        return (false, Some(HookResult::PublishAclResult(PublishAclResult::Rejected(false))));
                    }
                };

                // Basic namespace validation
                let pattern = &self.context.config.topic_namespace_pattern;
                let expected_prefix = pattern.replace("{tenant_id}", &tenant_context.tenant_id);

                if !topic.starts_with(&expected_prefix.split('/').next().unwrap_or("")) {
                    warn!("Topic namespace violation in ACL check: {} for tenant {}",
                        topic, tenant_context.tenant_id);
                    return (false, Some(HookResult::PublishAclResult(PublishAclResult::Rejected(false))));
                }

                debug!("Publish ACL check passed for: {} topic: {}", client_id, topic);
                (true, Some(HookResult::PublishAclResult(PublishAclResult::Allow)))
            }

            // Handle message publish - full policy check and transformation
            Parameter::MessagePublish(session_opt, _from, publish) => {
                let session = match session_opt {
                    Some(s) => s,
                    None => {
                        warn!("MessagePublish hook called without session");
                        return (true, acc);
                    }
                };

                debug!("MessagePublish hook fired for: {:?} topic: {}",
                    session.id.client_id, publish.topic);

                let client_id = session.id.client_id.as_ref();
                let topic: &str = &publish.topic;
                let qos = publish.qos as u8;
                let retain = publish.retain;
                let payload = publish.payload.as_ref();

                match self.handler.handle_message_publish(
                    client_id,
                    topic,
                    qos,
                    retain,
                    payload,
                ).await {
                    Ok(Some(transformed_payload)) => {
                        debug!("Message transformed for: {} topic: {}", client_id, topic);

                        // Create new publish with transformed payload
                        let mut new_publish = (*publish).clone();
                        new_publish.payload = transformed_payload.into();

                        (true, Some(HookResult::Publish(new_publish)))
                    }
                    Ok(None) => {
                        debug!("Message allowed without transformation: {} topic: {}", client_id, topic);
                        (true, acc)
                    }
                    Err(e) => {
                        warn!("Message publish rejected: {} topic: {} - {}", client_id, topic, e);
                        (false, acc)
                    }
                }
            }

            // Handle subscribe ACL check
            Parameter::ClientSubscribeCheckAcl(session, subscribe) => {
                debug!("ClientSubscribeCheckAcl hook fired for: {:?} topic: {}",
                    session.id.client_id, subscribe.topic_filter);

                let client_id = session.id.client_id.as_ref();
                let topic_filter: &str = &subscribe.topic_filter;
                let qos = subscribe.opts.qos() as u8;

                match self.handler.handle_client_subscribe(client_id, topic_filter, qos).await {
                    Ok(_) => {
                        debug!("Subscribe allowed: {} topic: {}", client_id, topic_filter);
                        (true, Some(HookResult::SubscribeAclResult(
                            SubscribeAclResult::new_success(subscribe.opts.qos(), None)
                        )))
                    }
                    Err(e) => {
                        warn!("Subscribe rejected: {} topic: {} - {}", client_id, topic_filter, e);
                        (false, None)
                    }
                }
            }

            // Pass through other hook types
            _ => (true, acc),
        }
    }
}
