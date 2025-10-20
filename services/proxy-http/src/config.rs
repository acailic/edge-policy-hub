use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Listen host address
    pub host: String,

    /// Listen port
    pub port: u16,

    /// Upstream backend URL
    pub upstream_url: String,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Maximum body size in bytes
    pub max_body_size_bytes: usize,

    /// OPA enforcer service URL
    pub enforcer_url: String,

    /// Enable mTLS client authentication
    pub enable_mtls: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,

    /// TLS private key path
    pub tls_key_path: Option<PathBuf>,

    /// Client CA certificate path for mTLS
    pub tls_client_ca_path: Option<PathBuf>,

    /// Enable JWT authentication
    pub enable_jwt: bool,

    /// JWT shared secret for HS256 (optional)
    pub jwt_secret: Option<String>,

    /// JWT public key path for RS256 (optional)
    pub jwt_public_key_path: Option<PathBuf>,

    /// Expected JWT issuer
    pub jwt_issuer: Option<String>,

    /// Expected JWT audience
    pub jwt_audience: Option<String>,

    /// JWT algorithm (HS256, RS256, ES256)
    pub jwt_algorithm: JwtAlgorithm,

    /// Forward Authorization header to upstream
    pub forward_auth_header: bool,

    /// Log level
    pub log_level: String,

    /// Quota tracker service URL
    pub quota_tracker_url: Option<String>,

    /// Quota tracker API token
    pub quota_tracker_token: Option<String>,

    /// Default region for requests
    pub default_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
}

impl Default for JwtAlgorithm {
    fn default() -> Self {
        JwtAlgorithm::RS256
    }
}

impl std::str::FromStr for JwtAlgorithm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "HS256" => Ok(JwtAlgorithm::HS256),
            "HS384" => Ok(JwtAlgorithm::HS384),
            "HS512" => Ok(JwtAlgorithm::HS512),
            "RS256" => Ok(JwtAlgorithm::RS256),
            "RS384" => Ok(JwtAlgorithm::RS384),
            "RS512" => Ok(JwtAlgorithm::RS512),
            "ES256" => Ok(JwtAlgorithm::ES256),
            "ES384" => Ok(JwtAlgorithm::ES384),
            _ => anyhow::bail!("Unsupported JWT algorithm: {}", s),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("PROXY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PROXY_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("Invalid PROXY_PORT")?;

        let upstream_url =
            std::env::var("UPSTREAM_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

        let request_timeout_secs = std::env::var("REQUEST_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .context("Invalid REQUEST_TIMEOUT_SECS")?;

        let max_body_size_bytes = std::env::var("MAX_BODY_SIZE_BYTES")
            .unwrap_or_else(|_| "10485760".to_string()) // 10MB
            .parse()
            .context("Invalid MAX_BODY_SIZE_BYTES")?;

        let enforcer_url =
            std::env::var("ENFORCER_URL").unwrap_or_else(|_| "http://127.0.0.1:8181".to_string());

        let enable_mtls = std::env::var("ENABLE_MTLS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .context("Invalid ENABLE_MTLS")?;

        let tls_cert_path = std::env::var("TLS_CERT_PATH").ok().map(PathBuf::from);

        let tls_key_path = std::env::var("TLS_KEY_PATH").ok().map(PathBuf::from);

        let tls_client_ca_path = std::env::var("TLS_CLIENT_CA_PATH").ok().map(PathBuf::from);

        let enable_jwt = std::env::var("ENABLE_JWT")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .context("Invalid ENABLE_JWT")?;

        let jwt_secret = std::env::var("JWT_SECRET").ok();

        let jwt_public_key_path = std::env::var("JWT_PUBLIC_KEY_PATH").ok().map(PathBuf::from);

        let jwt_issuer = std::env::var("JWT_ISSUER").ok();

        let jwt_audience = std::env::var("JWT_AUDIENCE").ok();

        let jwt_algorithm = std::env::var("JWT_ALGORITHM")
            .unwrap_or_else(|_| "RS256".to_string())
            .parse()?;

        let forward_auth_header = std::env::var("FORWARD_AUTH_HEADER")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .context("Invalid FORWARD_AUTH_HEADER")?;

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let quota_tracker_url = std::env::var("QUOTA_TRACKER_URL").ok();

        let quota_tracker_token = std::env::var("QUOTA_TRACKER_TOKEN").ok();

        let default_region = std::env::var("DEFAULT_REGION").ok();

        Ok(Self {
            host,
            port,
            upstream_url,
            request_timeout_secs,
            max_body_size_bytes,
            enforcer_url,
            enable_mtls,
            tls_cert_path,
            tls_key_path,
            tls_client_ca_path,
            enable_jwt,
            jwt_secret,
            jwt_public_key_path,
            jwt_issuer,
            jwt_audience,
            jwt_algorithm,
            forward_auth_header,
            log_level,
            quota_tracker_url,
            quota_tracker_token,
            default_region,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate TLS configuration
        if self.enable_mtls {
            if self.tls_cert_path.is_none() {
                anyhow::bail!("TLS_CERT_PATH is required when ENABLE_MTLS is true");
            }
            if self.tls_key_path.is_none() {
                anyhow::bail!("TLS_KEY_PATH is required when ENABLE_MTLS is true");
            }
            if self.tls_client_ca_path.is_none() {
                anyhow::bail!("TLS_CLIENT_CA_PATH is required when ENABLE_MTLS is true");
            }

            // Verify files exist
            if let Some(ref path) = self.tls_cert_path {
                if !path.exists() {
                    anyhow::bail!("TLS certificate file not found: {:?}", path);
                }
            }
            if let Some(ref path) = self.tls_key_path {
                if !path.exists() {
                    anyhow::bail!("TLS key file not found: {:?}", path);
                }
            }
            if let Some(ref path) = self.tls_client_ca_path {
                if !path.exists() {
                    anyhow::bail!("TLS client CA file not found: {:?}", path);
                }
            }
        }

        // Validate JWT configuration
        if self.enable_jwt {
            match self.jwt_algorithm {
                JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512 => {
                    if self.jwt_secret.is_none() {
                        anyhow::bail!(
                            "JWT_SECRET is required for HMAC algorithms (HS256/HS384/HS512)"
                        );
                    }
                }
                JwtAlgorithm::RS256
                | JwtAlgorithm::RS384
                | JwtAlgorithm::RS512
                | JwtAlgorithm::ES256
                | JwtAlgorithm::ES384 => {
                    if self.jwt_public_key_path.is_none() {
                        anyhow::bail!("JWT_PUBLIC_KEY_PATH is required for RSA/ECDSA algorithms");
                    }
                    if let Some(ref path) = self.jwt_public_key_path {
                        if !path.exists() {
                            anyhow::bail!("JWT public key file not found: {:?}", path);
                        }
                    }
                }
            }
        }

        // Validate upstream URL
        if self.upstream_url.is_empty() {
            anyhow::bail!("UPSTREAM_URL cannot be empty");
        }

        // Validate enforcer URL
        if self.enforcer_url.is_empty() {
            anyhow::bail!("ENFORCER_URL cannot be empty");
        }

        // Validate timeout
        if self.request_timeout_secs == 0 {
            anyhow::bail!("REQUEST_TIMEOUT_SECS must be greater than 0");
        }

        // Validate max body size
        if self.max_body_size_bytes == 0 {
            anyhow::bail!("MAX_BODY_SIZE_BYTES must be greater than 0");
        }

        // Validate quota tracker configuration
        match (
            self.quota_tracker_url.as_ref(),
            self.quota_tracker_token.as_ref(),
        ) {
            (Some(_), Some(_)) => {}
            (Some(_), None) => {
                anyhow::bail!(
                    "QUOTA_TRACKER_TOKEN is required when QUOTA_TRACKER_URL is configured"
                )
            }
            (None, Some(_)) => {
                anyhow::bail!("QUOTA_TRACKER_URL must be set when QUOTA_TRACKER_TOKEN is provided")
            }
            (None, None) => {}
        }

        Ok(())
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Get the listen address
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_algorithm_from_str() {
        assert_eq!(
            "RS256".parse::<JwtAlgorithm>().unwrap(),
            JwtAlgorithm::RS256
        );
        assert_eq!(
            "HS256".parse::<JwtAlgorithm>().unwrap(),
            JwtAlgorithm::HS256
        );
        assert_eq!(
            "ES256".parse::<JwtAlgorithm>().unwrap(),
            JwtAlgorithm::ES256
        );
        assert!("INVALID".parse::<JwtAlgorithm>().is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = ProxyConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            upstream_url: "http://localhost:8000".to_string(),
            request_timeout_secs: 30,
            max_body_size_bytes: 10485760,
            enforcer_url: "http://localhost:8181".to_string(),
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
            log_level: "info".to_string(),
            quota_tracker_url: None,
            quota_tracker_token: None,
            default_region: None,
        };

        // Valid configuration
        assert!(config.validate().is_ok());

        // Invalid: empty upstream URL
        config.upstream_url = "".to_string();
        assert!(config.validate().is_err());
        config.upstream_url = "http://localhost:8000".to_string();

        // Invalid: zero timeout
        config.request_timeout_secs = 0;
        assert!(config.validate().is_err());
        config.request_timeout_secs = 30;

        // Invalid: mTLS enabled without cert
        config.enable_mtls = true;
        assert!(config.validate().is_err());
        config.enable_mtls = false;

        // Invalid: quota URL without token
        config.quota_tracker_url = Some("http://quota.local".to_string());
        assert!(config.validate().is_err());

        // Valid: quota URL with token
        config.quota_tracker_token = Some("secret".to_string());
        assert!(config.validate().is_ok());

        // Invalid: quota token without URL
        config.quota_tracker_url = None;
        assert!(config.validate().is_err());
    }
}
