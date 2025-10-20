use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tracing::debug;

use super::QuotaError;

#[derive(Debug, Clone)]
pub struct QuotaMetrics {
    pub message_count: u64,
    pub bytes_sent: u64,
    pub last_reset: DateTime<Utc>,
}

impl Default for QuotaMetrics {
    fn default() -> Self {
        Self {
            message_count: 0,
            bytes_sent: 0,
            last_reset: Utc::now(),
        }
    }
}

pub struct QuotaTracker {
    metrics: Arc<DashMap<String, QuotaMetrics>>,
    message_limit: u64,
    bandwidth_limit_bytes: u64,
}

impl QuotaTracker {
    pub fn new(message_limit: u64, bandwidth_limit_gb: f64) -> Self {
        let bandwidth_limit_bytes = (bandwidth_limit_gb * 1_073_741_824.0) as u64; // Convert GB to bytes

        Self {
            metrics: Arc::new(DashMap::new()),
            message_limit,
            bandwidth_limit_bytes,
        }
    }

    pub fn increment_message_count(&self, tenant_id: &str, payload_size: usize) -> QuotaMetrics {
        let mut entry = self.metrics.entry(tenant_id.to_string()).or_default();

        // Check if daily reset needed
        let now = Utc::now();
        let last_reset = entry.last_reset;
        let days_since_reset = (now - last_reset).num_days();

        if days_since_reset >= 1 {
            // Reset counters for new day
            debug!("Resetting quota counters for tenant: {}", tenant_id);
            entry.message_count = 0;
            entry.bytes_sent = 0;
            entry.last_reset = now;
        }

        // Increment counters
        entry.message_count += 1;
        entry.bytes_sent += payload_size as u64;

        debug!(
            "Incremented quota for tenant '{}': messages={}, bytes={}",
            tenant_id, entry.message_count, entry.bytes_sent
        );

        entry.clone()
    }

    pub fn get_metrics(&self, tenant_id: &str) -> Option<QuotaMetrics> {
        self.metrics.get(tenant_id).map(|entry| entry.clone())
    }

    pub fn check_quota(&self, tenant_id: &str) -> Result<(), QuotaError> {
        if let Some(metrics) = self.get_metrics(tenant_id) {
            // Check message count limit
            if metrics.message_count >= self.message_limit {
                return Err(QuotaError::LimitExceeded {
                    tenant_id: tenant_id.to_string(),
                    limit: self.message_limit,
                    current: metrics.message_count,
                });
            }

            // Check bandwidth limit
            if metrics.bytes_sent >= self.bandwidth_limit_bytes {
                return Err(QuotaError::LimitExceeded {
                    tenant_id: tenant_id.to_string(),
                    limit: self.bandwidth_limit_bytes,
                    current: metrics.bytes_sent,
                });
            }
        }

        Ok(())
    }

    pub fn reset_tenant_quota(&self, tenant_id: &str) {
        if let Some(mut entry) = self.metrics.get_mut(tenant_id) {
            entry.message_count = 0;
            entry.bytes_sent = 0;
            entry.last_reset = Utc::now();
            debug!("Manually reset quota for tenant: {}", tenant_id);
        }
    }
}
