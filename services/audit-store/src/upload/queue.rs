use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, MissedTickBehavior};
use tracing::{debug, info, warn};

use crate::config::AuditStoreConfig;
use crate::storage::{AuditDatabase, TenantRegistry};

use super::error::UploadError;

#[derive(Clone)]
pub struct UploadQueue {
    database: Arc<AuditDatabase>,
    tenant_registry: Arc<TenantRegistry>,
    http_client: Client,
    upload_endpoint: Option<String>,
    batch_size: usize,
    upload_interval: Duration,
}

impl UploadQueue {
    pub fn new(
        database: Arc<AuditDatabase>,
        tenant_registry: Arc<TenantRegistry>,
        config: &AuditStoreConfig,
    ) -> Self {
        let client = Client::builder()
            .user_agent("edge-policy-audit-store/0.1.0")
            .build()
            .expect("failed to build HTTP client");

        Self {
            database,
            tenant_registry,
            http_client: client,
            upload_endpoint: config.upload_endpoint.clone(),
            batch_size: config.upload_batch_size,
            upload_interval: Duration::from_secs(config.upload_interval_secs),
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(self.upload_interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;

                match self.process_uploads().await {
                    Ok(count) if count > 0 => {
                        info!(count, "uploaded audit logs");
                    }
                    Ok(_) => {
                        debug!("no audit logs pending upload");
                    }
                    Err(err) => {
                        warn!(error = %err, "failed to process audit log upload batch");
                    }
                }
            }
        })
    }

    pub async fn process_uploads(&self) -> Result<usize, UploadError> {
        let endpoint = match &self.upload_endpoint {
            Some(endpoint) => endpoint.clone(),
            None => {
                debug!("upload endpoint not configured; skipping upload cycle");
                return Ok(0);
            }
        };

        let tenants = self
            .tenant_registry
            .list_tenants(Some("active"))
            .map_err(UploadError::from)?;

        let mut uploaded_total = 0usize;

        for tenant in tenants {
            let logs = self
                .database
                .get_unuploaded_logs(&tenant.tenant_id, self.batch_size)
                .map_err(UploadError::from)?;

            if logs.is_empty() {
                continue;
            }

            if let Err(err) = self
                .upload_batch(&endpoint, &tenant.tenant_id, &logs)
                .await
            {
                warn!(
                    tenant_id = %tenant.tenant_id,
                    error = %err,
                    "failed to upload audit batch"
                );
                continue;
            }

            let log_ids: Vec<String> = logs.iter().map(|log| log.log_id.clone()).collect();
            self.database
                .mark_logs_uploaded(&tenant.tenant_id, &log_ids)
                .map_err(UploadError::from)?;
            uploaded_total += log_ids.len();
        }

        Ok(uploaded_total)
    }

    async fn upload_batch(
        &self,
        endpoint: &str,
        tenant_id: &str,
        logs: &[crate::api::types::AuditLogEntry],
    ) -> Result<(), UploadError> {
        if logs.is_empty() {
            return Ok(());
        }

        let url = format!(
            "{}/tenants/{}/audit-logs",
            endpoint.trim_end_matches('/'),
            tenant_id
        );

        // Exponential backoff configuration
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF_MS: u64 = 100;
        const MAX_BACKOFF_MS: u64 = 5000;

        let mut attempt = 0;
        let mut backoff = INITIAL_BACKOFF_MS;

        loop {
            let response = self.http_client.post(&url).json(&logs).send().await?;

            if response.status().is_success() {
                debug!(
                    tenant_id = %tenant_id,
                    count = logs.len(),
                    attempts = attempt + 1,
                    "uploaded audit logs batch"
                );
                return Ok(());
            } else if response.status().is_server_error() {
                // Retry on server errors with exponential backoff
                if attempt < MAX_RETRIES {
                    // Add jitter (0-50% of backoff value)
                    let jitter = (rand::random::<f64>() * 0.5 * backoff as f64) as u64;
                    let sleep_duration = backoff + jitter;

                    debug!(
                        tenant_id = %tenant_id,
                        attempt = attempt + 1,
                        backoff_ms = sleep_duration,
                        status = %response.status(),
                        "retrying upload after server error"
                    );

                    sleep(Duration::from_millis(sleep_duration)).await;

                    attempt += 1;
                    backoff = (backoff * 2).min(MAX_BACKOFF_MS);
                } else {
                    return Err(UploadError::NetworkError(format!(
                        "server error {} after {} retries",
                        response.status(),
                        MAX_RETRIES
                    )));
                }
            } else if response.status().is_client_error() {
                // Don't retry on client errors
                return Err(UploadError::AuthenticationError);
            } else {
                return Err(UploadError::NetworkError(format!(
                    "unexpected response {}",
                    response.status()
                )));
            }
        }
    }
}
