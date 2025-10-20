use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::Url;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub audit_store_url: String,
    pub quota_tracker_url: String,
    pub enforcer_url: String,
    pub enforcer_host: String,
    pub enforcer_port: u16,
    pub enforcer_use_tls: bool,
    pub enforcer_bundles_dir: PathBuf,
    pub request_timeout_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            audit_store_url: "http://127.0.0.1:8182".to_string(),
            quota_tracker_url: "http://127.0.0.1:8183".to_string(),
            enforcer_url: "http://127.0.0.1:8181".to_string(),
            enforcer_host: "127.0.0.1".to_string(),
            enforcer_port: 8181,
            enforcer_use_tls: false,
            enforcer_bundles_dir: Self::default_bundles_dir(),
            request_timeout_secs: 10,
        }
    }
}

impl ServiceConfig {
    /// Returns the default absolute path for the enforcer bundles directory.
    /// Falls back to current directory + config/tenants.d if app config dir cannot be determined.
    fn default_bundles_dir() -> PathBuf {
        // Try to get the app config directory
        let config_dir = dirs::config_dir()
            .map(|dir| dir.join("edge-policy-hub"))
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        config_dir.join("config").join("tenants.d")
    }

    pub fn from_env() -> Result<Self> {
        let mut config = ServiceConfig::default();

        if let Ok(value) = env::var("AUDIT_STORE_URL") {
            config.audit_store_url = value;
        }

        if let Ok(value) = env::var("QUOTA_TRACKER_URL") {
            config.quota_tracker_url = value;
        }

        if let Ok(value) = env::var("ENFORCER_URL") {
            config.enforcer_url = value;
        }

        let mut enforcer_url = Url::parse(&config.enforcer_url)
            .map_err(|err| anyhow!("invalid ENFORCER_URL `{}`: {err}", config.enforcer_url))?;

        let mut resolved_host = enforcer_url
            .host_str()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let mut resolved_port = enforcer_url.port_or_known_default().unwrap_or(8181);
        let mut resolved_use_tls = enforcer_url.scheme() == "https";

        if let Ok(value) = env::var("ENFORCER_HOST") {
            if !value.trim().is_empty() {
                resolved_host = value;
            }
        }

        if let Ok(value) = env::var("ENFORCER_PORT") {
            resolved_port = value
                .parse::<u16>()
                .map_err(|err| anyhow!("invalid ENFORCER_PORT `{value}`: {err}"))?;
        }

        if let Ok(value) =
            env::var("ENFORCER_TLS_ENABLED").or_else(|_| env::var("ENFORCER_USE_TLS"))
        {
            resolved_use_tls = parse_bool(&value)
                .map_err(|err| anyhow!("invalid ENFORCER_TLS flag `{value}`: {err}"))?;
        }

        enforcer_url
            .set_scheme(if resolved_use_tls { "https" } else { "http" })
            .map_err(|_| anyhow!("failed to update ENFORCER_URL scheme"))?;
        enforcer_url
            .set_host(Some(&resolved_host))
            .map_err(|_| anyhow!("invalid ENFORCER_HOST `{}`", resolved_host))?;
        let port_update = if (resolved_use_tls && resolved_port == 443)
            || (!resolved_use_tls && resolved_port == 80)
        {
            enforcer_url.set_port(None)
        } else {
            enforcer_url.set_port(Some(resolved_port))
        };
        port_update.map_err(|_| anyhow!("invalid ENFORCER_PORT `{resolved_port}`"))?;

        config.enforcer_url = enforcer_url.to_string();
        config.enforcer_host = resolved_host;
        config.enforcer_port = resolved_port;
        config.enforcer_use_tls = resolved_use_tls;

        if let Ok(value) = env::var("ENFORCER_BUNDLES_DIR") {
            let path = PathBuf::from(&value);
            // Normalize to absolute path if relative
            config.enforcer_bundles_dir = if path.is_absolute() {
                path
            } else {
                env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            };
        }

        if let Ok(value) = env::var("REQUEST_TIMEOUT_SECS") {
            config.request_timeout_secs = value
                .parse::<u64>()
                .map_err(|err| anyhow!("invalid REQUEST_TIMEOUT_SECS value `{value}`: {err}",))?;
        }

        config.validate()?;

        // Log the resolved bundles directory
        info!(
          bundles_dir = %config.enforcer_bundles_dir.display(),
          "Tauri backend bundles directory resolved"
        );

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("AUDIT_STORE_URL", &self.audit_store_url),
            ("QUOTA_TRACKER_URL", &self.quota_tracker_url),
            ("ENFORCER_URL", &self.enforcer_url),
        ] {
            Url::parse(value).map_err(|err| anyhow!("invalid {name} `{value}`: {err}"))?;
        }

        if self.enforcer_host.trim().is_empty() {
            return Err(anyhow!("ENFORCER_HOST must not be empty"));
        }

        if self.enforcer_port == 0 {
            return Err(anyhow!("ENFORCER_PORT must be greater than zero"));
        }

        if Duration::from_secs(self.request_timeout_secs).is_zero() {
            return Err(anyhow!("REQUEST_TIMEOUT_SECS must be greater than zero"));
        }

        // Validate enforcer bundles directory only if it exists
        // Directory will be created lazily in write_policy_bundle_file when needed
        let bundles_dir = &self.enforcer_bundles_dir;
        if bundles_dir.exists() {
            let metadata = fs::metadata(bundles_dir).map_err(|err| {
                anyhow!(
                    "unable to read enforcer bundles directory `{}`: {err}",
                    bundles_dir.display()
                )
            })?;

            if !metadata.is_dir() {
                return Err(anyhow!(
                    "enforcer bundles path `{}` is not a directory",
                    bundles_dir.display()
                ));
            }

            if metadata.permissions().readonly() {
                return Err(anyhow!(
                    "enforcer bundles directory `{}` is not writable",
                    bundles_dir.display()
                ));
            }
        }

        Ok(())
    }
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        other => Err(anyhow!("invalid boolean value `{other}`")),
    }
}
