use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct QuotaTrackerConfig {
    pub server_host: String,
    pub server_port: u16,
    pub data_dir: PathBuf,
    pub persistence_interval_secs: u64,
    pub default_message_limit: u64,
    pub default_bandwidth_limit_gb: f64,
    pub enable_auto_reset: bool,
    pub log_level: String,
}

impl Default for QuotaTrackerConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8183,
            data_dir: PathBuf::from("data/quota"),
            persistence_interval_secs: 60,
            default_message_limit: 50_000,
            default_bandwidth_limit_gb: 100.0,
            enable_auto_reset: true,
            log_level: "info".to_string(),
        }
    }
}

impl QuotaTrackerConfig {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self::default();

        if let Ok(host) = env::var("QUOTA_HOST") {
            cfg.server_host = host;
        }
        if let Ok(port) = env::var("QUOTA_PORT") {
            cfg.server_port = port.parse().context("QUOTA_PORT must be a valid u16")?;
        }
        if let Ok(dir) = env::var("QUOTA_DATA_DIR") {
            cfg.data_dir = PathBuf::from(dir);
        }
        if let Ok(interval) = env::var("PERSISTENCE_INTERVAL_SECS") {
            cfg.persistence_interval_secs = interval
                .parse()
                .context("PERSISTENCE_INTERVAL_SECS must be a positive integer")?;
        }
        if let Ok(limit) = env::var("DEFAULT_MESSAGE_LIMIT") {
            cfg.default_message_limit = limit
                .parse()
                .context("DEFAULT_MESSAGE_LIMIT must be a positive integer")?;
        }
        if let Ok(limit) = env::var("DEFAULT_BANDWIDTH_LIMIT_GB") {
            cfg.default_bandwidth_limit_gb = limit
                .parse()
                .context("DEFAULT_BANDWIDTH_LIMIT_GB must be a floating point number")?;
        }
        if let Ok(flag) = env::var("ENABLE_AUTO_RESET") {
            cfg.enable_auto_reset = parse_bool(&flag)
                .with_context(|| format!("ENABLE_AUTO_RESET is invalid: {flag}"))?;
        }
        if let Ok(level) = env::var("LOG_LEVEL") {
            cfg.log_level = level;
        }

        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        ensure_directory(&self.data_dir)?;

        if self.default_message_limit == 0 {
            anyhow::bail!("DEFAULT_MESSAGE_LIMIT must be greater than zero");
        }
        if self.default_bandwidth_limit_gb <= 0.0 {
            anyhow::bail!("DEFAULT_BANDWIDTH_LIMIT_GB must be greater than zero");
        }
        if self.persistence_interval_secs == 0 {
            anyhow::bail!("PERSISTENCE_INTERVAL_SECS must be greater than zero");
        }

        Ok(())
    }
}

fn ensure_directory(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            anyhow::bail!("{} exists but is not a directory", path.display());
        }
    } else {
        fs::create_dir_all(path)
            .with_context(|| format!("unable to create data directory {}", path.display()))?;
    }
    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => anyhow::bail!("invalid boolean value {value}"),
    }
}
