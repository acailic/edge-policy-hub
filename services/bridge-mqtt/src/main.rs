use edge_policy_bridge_mqtt::{broker::MqttBroker, config::BridgeConfig};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment
    let config = BridgeConfig::from_env()?;

    // Initialize tracing with configured log level
    init_tracing(&config.log_level);

    info!("edge-policy-bridge-mqtt service starting");
    info!("Configuration:");
    info!("  Broker: {}:{}", config.broker_host, config.broker_port);
    info!("  Broker name: {}", config.broker_name);
    info!("  TLS enabled: {}", config.enable_tls);
    info!("  mTLS enabled: {}", config.enable_mtls);
    if config.enable_mtls {
        info!("  Certificate CN as username: {}", config.cert_cn_as_username);
    }
    info!("  Enforcer URL: {}", config.enforcer_url);
    info!("  Use MQTT endpoints: {}", config.use_mqtt_endpoints);
    info!("  Topic namespace pattern: {}", config.topic_namespace_pattern);
    info!("  Allow wildcard subscriptions: {}", config.allow_wildcard_subscriptions);
    info!("  Max payload size: {} bytes", config.max_payload_size_bytes);
    info!("  Payload transformation enabled: {}", config.enable_payload_transformation);
    info!("  Request timeout: {} seconds", config.request_timeout_secs);
    info!("  Message limit: {} msg/day", config.message_limit);
    info!("  Bandwidth limit: {} GB/day", config.bandwidth_limit_gb);

    // Validate configuration
    config.validate()?;
    info!("Configuration validated successfully");

    // Create and run the MQTT broker
    let broker = MqttBroker::new(config.clone())?;
    info!("MQTT broker initialized successfully");

    broker.run().await?;

    info!("edge-policy-bridge-mqtt service stopped");
    Ok(())
}

fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
