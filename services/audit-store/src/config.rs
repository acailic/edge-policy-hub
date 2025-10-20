use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditStoreConfig {
    pub server_host: String,
    pub server_port: u16,
    pub data_dir: PathBuf,
    pub hmac_secret_key: String,
    pub enable_deferred_upload: bool,
    pub upload_batch_size: usize,
    pub upload_interval_secs: u64,
    pub upload_endpoint: Option<String>,
    pub max_log_age_days: u64,
    pub log_level: String,
}

impl Default for AuditStoreConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8182,
            data_dir: PathBuf::from("data/audit"),
            hmac_secret_key: String::new(),
            enable_deferred_upload: true,
            upload_batch_size: 1_000,
            upload_interval_secs: 300,
            upload_endpoint: None,
            max_log_age_days: 90,
            log_level: "info".to_string(),
        }
    }
}

impl AuditStoreConfig {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self::default();

        if let Ok(host) = env::var("AUDIT_HOST") {
            cfg.server_host = host;
        }
        if let Ok(port) = env::var("AUDIT_PORT") {
            cfg.server_port = port.parse().context("AUDIT_PORT must be a valid u16")?;
        }
        if let Ok(dir) = env::var("AUDIT_DATA_DIR") {
            cfg.data_dir = PathBuf::from(dir);
        }
        cfg.hmac_secret_key = env::var("AUDIT_HMAC_SECRET").unwrap_or_else(|_| generate_secret());

        if let Ok(flag) = env::var("ENABLE_DEFERRED_UPLOAD") {
            cfg.enable_deferred_upload = parse_bool(&flag)
                .with_context(|| format!("ENABLE_DEFERRED_UPLOAD is invalid: {flag}"))?;
        }
        if let Ok(size) = env::var("UPLOAD_BATCH_SIZE") {
            cfg.upload_batch_size =
                size.parse().context("UPLOAD_BATCH_SIZE must be a positive integer")?;
        }
        if let Ok(interval) = env::var("UPLOAD_INTERVAL_SECS") {
            cfg.upload_interval_secs = interval
                .parse()
                .context("UPLOAD_INTERVAL_SECS must be a positive integer")?;
        }
        if let Ok(endpoint) = env::var("UPLOAD_ENDPOINT") {
            cfg.upload_endpoint = if endpoint.is_empty() {
                None
            } else {
                Some(endpoint)
            };
        }
        if let Ok(age) = env::var("MAX_LOG_AGE_DAYS") {
            cfg.max_log_age_days =
                age.parse().context("MAX_LOG_AGE_DAYS must be a positive integer")?;
        }
        if let Ok(level) = env::var("LOG_LEVEL") {
            cfg.log_level = level;
        }

        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        ensure_directory(&self.data_dir)?;

        if self.hmac_secret_key.trim().is_empty() {
            anyhow::bail!("AUDIT_HMAC_SECRET must be provided or auto-generated");
        }
        if self.upload_batch_size == 0 {
            anyhow::bail!("UPLOAD_BATCH_SIZE must be greater than zero");
        }
        if self.upload_interval_secs == 0 {
            anyhow::bail!("UPLOAD_INTERVAL_SECS must be greater than zero");
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

fn generate_secret() -> String {
    let seed = format!(
        "{}:{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        Uuid::new_v4()
    );
    base64::encode(seed)
}
