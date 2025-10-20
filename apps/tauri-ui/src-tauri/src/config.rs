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
    pub enforcer_bundles_dir: PathBuf,
    pub request_timeout_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            audit_store_url: "http://127.0.0.1:8182".to_string(),
            quota_tracker_url: "http://127.0.0.1:8183".to_string(),
            enforcer_url: "http://127.0.0.1:8181".to_string(),
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
