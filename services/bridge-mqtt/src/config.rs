use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub broker_name: String,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub tls_client_ca_path: Option<PathBuf>,
    pub enable_mtls: bool,
    pub cert_cn_as_username: bool,
    pub enforcer_url: String,
    pub topic_namespace_pattern: String,
    pub allow_wildcard_subscriptions: bool,
    pub max_payload_size_bytes: usize,
    pub enable_payload_transformation: bool,
    pub request_timeout_secs: u64,
    pub log_level: String,
    pub use_mqtt_endpoints: bool,
    pub message_limit: u64,
    pub bandwidth_limit_gb: f64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            broker_host: "0.0.0.0".to_string(),
            broker_port: 1883,
            broker_name: "edge-policy-mqtt-broker".to_string(),
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_client_ca_path: None,
            enable_mtls: false,
            cert_cn_as_username: false,
            enforcer_url: "http://localhost:8181".to_string(),
            topic_namespace_pattern: "{tenant_id}/#".to_string(),
            allow_wildcard_subscriptions: true,
            max_payload_size_bytes: 1_048_576, // 1MB
            enable_payload_transformation: true,
            request_timeout_secs: 5,
            log_level: "info".to_string(),
            use_mqtt_endpoints: false,
            message_limit: 10000,
            bandwidth_limit_gb: 1.0,
        }
    }
}

impl BridgeConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(host) = std::env::var("MQTT_HOST") {
            config.broker_host = host;
        }

        if let Ok(port) = std::env::var("MQTT_PORT") {
            config.broker_port = port.parse().context("Invalid MQTT_PORT")?;
        }

        if let Ok(name) = std::env::var("MQTT_BROKER_NAME") {
            config.broker_name = name;
        }

        if let Ok(enable_tls) = std::env::var("ENABLE_TLS") {
            config.enable_tls = enable_tls.eq_ignore_ascii_case("true") || enable_tls == "1";
        }

        if let Ok(cert_path) = std::env::var("TLS_CERT_PATH") {
            config.tls_cert_path = Some(PathBuf::from(cert_path));
        }

        if let Ok(key_path) = std::env::var("TLS_KEY_PATH") {
            config.tls_key_path = Some(PathBuf::from(key_path));
        }

        if let Ok(ca_path) = std::env::var("TLS_CLIENT_CA_PATH") {
            config.tls_client_ca_path = Some(PathBuf::from(ca_path));
        }

        if let Ok(enable_mtls) = std::env::var("ENABLE_MTLS") {
            config.enable_mtls = enable_mtls.eq_ignore_ascii_case("true") || enable_mtls == "1";
        }

        if let Ok(cert_cn) = std::env::var("CERT_CN_AS_USERNAME") {
            config.cert_cn_as_username = cert_cn.eq_ignore_ascii_case("true") || cert_cn == "1";
        }

        if let Ok(url) = std::env::var("ENFORCER_URL") {
            config.enforcer_url = url;
        }

        if let Ok(pattern) = std::env::var("TOPIC_NAMESPACE_PATTERN") {
            config.topic_namespace_pattern = pattern;
        }

        if let Ok(allow_wildcards) = std::env::var("ALLOW_WILDCARD_SUBSCRIPTIONS") {
            config.allow_wildcard_subscriptions = allow_wildcards.eq_ignore_ascii_case("true") || allow_wildcards == "1";
        }

        if let Ok(max_size) = std::env::var("MAX_PAYLOAD_SIZE_BYTES") {
            config.max_payload_size_bytes = max_size.parse().context("Invalid MAX_PAYLOAD_SIZE_BYTES")?;
        }

        if let Ok(enable_transform) = std::env::var("ENABLE_PAYLOAD_TRANSFORMATION") {
            config.enable_payload_transformation = enable_transform.eq_ignore_ascii_case("true") || enable_transform == "1";
        }

        if let Ok(timeout) = std::env::var("REQUEST_TIMEOUT_SECS") {
            config.request_timeout_secs = timeout.parse().context("Invalid REQUEST_TIMEOUT_SECS")?;
        }

        if let Ok(log_level) = std::env::var("LOG_LEVEL") {
            config.log_level = log_level;
        }

        if let Ok(use_mqtt_endpoints) = std::env::var("USE_MQTT_ENDPOINTS") {
            config.use_mqtt_endpoints = use_mqtt_endpoints.eq_ignore_ascii_case("true") || use_mqtt_endpoints == "1";
        }

        if let Ok(limit) = std::env::var("MESSAGE_LIMIT") {
            config.message_limit = limit.parse().context("Invalid MESSAGE_LIMIT")?;
        }

        if let Ok(bw_limit) = std::env::var("BANDWIDTH_LIMIT_GB") {
            config.bandwidth_limit_gb = bw_limit.parse().context("Invalid BANDWIDTH_LIMIT_GB")?;
        }

        Ok(config)
    }

    pub fn validate(&self) -> Result<&Self> {
        // Validate mTLS implies TLS
        if self.enable_mtls && !self.enable_tls {
            anyhow::bail!("mTLS requires TLS to be enabled");
        }

        // Validate TLS files exist when TLS is enabled
        if self.enable_tls {
            if let Some(ref cert_path) = self.tls_cert_path {
                if !cert_path.exists() {
                    anyhow::bail!("TLS certificate file not found: {}", cert_path.display());
                }
            } else {
                anyhow::bail!("TLS enabled but TLS_CERT_PATH not provided");
            }

            if let Some(ref key_path) = self.tls_key_path {
                if !key_path.exists() {
                    anyhow::bail!("TLS key file not found: {}", key_path.display());
                }
            } else {
                anyhow::bail!("TLS enabled but TLS_KEY_PATH not provided");
            }
        }

        // Validate mTLS client CA exists when mTLS is enabled
        if self.enable_mtls {
            if let Some(ref ca_path) = self.tls_client_ca_path {
                if !ca_path.exists() {
                    anyhow::bail!("mTLS client CA file not found: {}", ca_path.display());
                }
            } else {
                anyhow::bail!("mTLS enabled but TLS_CLIENT_CA_PATH not provided");
            }
        }

        // Validate enforcer URL format
        if !self.enforcer_url.starts_with("http://") && !self.enforcer_url.starts_with("https://") {
            anyhow::bail!("ENFORCER_URL must start with http:// or https://");
        }

        // Validate topic namespace pattern contains tenant_id placeholder
        if !self.topic_namespace_pattern.contains("{tenant_id}") {
            anyhow::bail!("TOPIC_NAMESPACE_PATTERN must contain {{tenant_id}} placeholder");
        }

        // Validate positive limits
        if self.max_payload_size_bytes == 0 {
            anyhow::bail!("MAX_PAYLOAD_SIZE_BYTES must be greater than 0");
        }

        if self.request_timeout_secs == 0 {
            anyhow::bail!("REQUEST_TIMEOUT_SECS must be greater than 0");
        }

        if self.message_limit == 0 {
            anyhow::bail!("MESSAGE_LIMIT must be greater than 0");
        }

        if self.bandwidth_limit_gb <= 0.0 {
            anyhow::bail!("BANDWIDTH_LIMIT_GB must be greater than 0");
        }

        Ok(self)
    }
}
