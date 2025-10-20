use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcerConfig {
    pub server_host: String,
    pub server_port: u16,
    pub bundles_dir: PathBuf,
    pub enable_hot_reload: bool,
    pub reload_interval_secs: u64,
    pub log_level: String,
}

impl Default for EnforcerConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8181,
            bundles_dir: Self::default_bundles_dir(),
            enable_hot_reload: true,
            reload_interval_secs: 5,
            log_level: "info".to_string(),
        }
    }
}

impl EnforcerConfig {
    /// Returns the default absolute path for the bundles directory.
    /// Falls back to current directory + config/tenants.d if config dir cannot be determined.
    fn default_bundles_dir() -> PathBuf {
        // Try to get the same config directory as Tauri
        let config_dir = dirs::config_dir()
            .map(|dir| dir.join("edge-policy-hub"))
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        config_dir.join("config").join("tenants.d")
    }

    pub fn from_env() -> Result<Self> {
        let mut config = EnforcerConfig::default();

        if let Ok(host) = env::var("ENFORCER_HOST") {
            if !host.trim().is_empty() {
                config.server_host = host;
            }
        }

        if let Ok(port) = env::var("ENFORCER_PORT") {
            config.server_port = port
                .parse::<u16>()
                .context("failed to parse ENFORCER_PORT as u16")?;
        }

        if let Ok(dir) = env::var("BUNDLES_DIR") {
            if !dir.trim().is_empty() {
                let path = PathBuf::from(&dir);
                // Normalize to absolute path if relative
                config.bundles_dir = if path.is_absolute() {
                    path
                } else {
                    env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(path)
                };
            }
        }

        if let Ok(flag) = env::var("ENABLE_HOT_RELOAD") {
            config.enable_hot_reload =
                parse_bool(&flag).context("failed to parse ENABLE_HOT_RELOAD as bool")?;
        }

        if let Ok(interval) = env::var("RELOAD_INTERVAL_SECS") {
            config.reload_interval_secs = interval
                .parse::<u64>()
                .context("failed to parse RELOAD_INTERVAL_SECS as u64")?;
        }

        if let Ok(level) = env::var("LOG_LEVEL") {
            if !level.trim().is_empty() {
                config.log_level = level;
            }
        }

        config.validate()?;

        // Log the resolved bundles directory
        info!(
            bundles_dir = %config.bundles_dir.display(),
            "Enforcer bundles directory resolved"
        );

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        validate_bundles_dir(&self.bundles_dir)?;
        Ok(())
    }
}

fn parse_bool(value: &str) -> Result<bool> {
    value.parse::<bool>().or_else(|_| match value {
        "1" => Ok(true),
        "0" => Ok(false),
        other => Err(anyhow!("invalid boolean value: {}", other)),
    })
}

fn validate_bundles_dir(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).with_context(|| {
        format!(
            "bundles directory '{}' does not exist or is not accessible",
            path.display()
        )
    })?;

    if !metadata.is_dir() {
        return Err(anyhow!(
            "bundles directory '{}' is not a directory",
            path.display()
        ));
    }

    Ok(())
}
