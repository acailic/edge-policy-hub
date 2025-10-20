use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Utc};
use dashmap::DashMap;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info};

use crate::config::QuotaTrackerConfig;
use crate::storage::{QuotaDatabase, StorageError};

use super::error::QuotaError;
use super::metrics::QuotaMetrics;
use super::{BANDWIDTH_QUOTA_TYPE, MESSAGE_QUOTA_TYPE};

#[derive(Clone)]
pub struct QuotaManager {
    cache: Arc<DashMap<String, QuotaMetrics>>,
    database: Arc<QuotaDatabase>,
    default_message_limit: u64,
    default_bandwidth_limit_gb: f64,
    persistence_interval: Duration,
    enable_auto_reset: bool,
}

impl QuotaManager {
    pub fn new(database: Arc<QuotaDatabase>, config: &QuotaTrackerConfig) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            database,
            default_message_limit: config.default_message_limit,
            default_bandwidth_limit_gb: config.default_bandwidth_limit_gb,
            persistence_interval: Duration::from_secs(config.persistence_interval_secs),
            enable_auto_reset: config.enable_auto_reset,
        }
    }

    pub fn load_from_database(&self) -> Result<usize, StorageError> {
        let limits = self.database.list_limits()?;
        let mut loaded = 0usize;
        let day_period = current_day_period();
        let month_period = current_month_period();

        for limit in limits {
            let message_used =
                self.database
                    .load_usage(&limit.tenant_id, &day_period, MESSAGE_QUOTA_TYPE)?;
            let bandwidth_used =
                self.database
                    .load_usage(&limit.tenant_id, &month_period, BANDWIDTH_QUOTA_TYPE)?;

            let metrics = QuotaMetrics {
                tenant_id: limit.tenant_id.clone(),
                message_count: message_used,
                bytes_sent: bandwidth_used,
                message_limit: limit.message_limit,
                bandwidth_limit_bytes: limit.bandwidth_limit_bytes,
                last_reset: Utc::now(),
                period: day_period.clone(),
            };

            self.cache.insert(limit.tenant_id, metrics);
            loaded += 1;
        }

        Ok(loaded)
    }

    pub fn increment_message_count(
        &self,
        tenant_id: &str,
        messages: u64,
        bytes: u64,
    ) -> QuotaMetrics {
        self.ensure_entry(tenant_id);

        let mut entry = self
            .cache
            .get_mut(tenant_id)
            .expect("entry must exist after ensure_entry");

        let now = Utc::now();
        let current_day = current_day_period();
        let current_month = current_month_period();
        let last_month = format!("{:04}-{:02}", entry.last_reset.year(), entry.last_reset.month());

        if self.enable_auto_reset && entry.period != current_day {
            entry.period = current_day.clone();
            entry.message_count = 0;
        }

        if self.enable_auto_reset && current_month != last_month {
            entry.bytes_sent = 0;
            entry.last_reset = now;
        }

        let msg_inc = if messages == 0 { 1 } else { messages };
        entry.message_count = entry.message_count.saturating_add(msg_inc);
        entry.bytes_sent = entry.bytes_sent.saturating_add(bytes);

        entry.clone()
    }

    pub fn get_metrics(&self, tenant_id: &str) -> Option<QuotaMetrics> {
        self.cache.get(tenant_id).map(|metrics| metrics.clone())
    }

    pub fn all_metrics(&self) -> Vec<QuotaMetrics> {
        self.cache
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn check_quota(&self, tenant_id: &str) -> Result<(), QuotaError> {
        let metrics = self
            .get_metrics(tenant_id)
            .ok_or_else(|| QuotaError::TenantNotFound(tenant_id.to_string()))?;

        if metrics.is_message_limit_exceeded() {
            return Err(QuotaError::LimitExceeded {
                tenant_id: tenant_id.to_string(),
                quota_type: MESSAGE_QUOTA_TYPE.to_string(),
                limit: metrics.message_limit,
                current: metrics.message_count,
            });
        }

        if metrics.is_bandwidth_limit_exceeded() {
            return Err(QuotaError::LimitExceeded {
                tenant_id: tenant_id.to_string(),
                quota_type: BANDWIDTH_QUOTA_TYPE.to_string(),
                limit: metrics.bandwidth_limit_bytes,
                current: metrics.bytes_sent,
            });
        }

        Ok(())
    }

    pub fn set_limits(
        &self,
        tenant_id: &str,
        message_limit: u64,
        bandwidth_limit_gb: f64,
    ) -> Result<(), QuotaError> {
        self.database
            .set_quota_limits(tenant_id, message_limit, bandwidth_limit_gb)?;

        let bytes_limit = (bandwidth_limit_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        self.ensure_entry(tenant_id);

        if let Some(mut metrics) = self.cache.get_mut(tenant_id) {
            metrics.message_limit = message_limit;
            metrics.bandwidth_limit_bytes = bytes_limit;
        }

        info!(
            tenant_id,
            message_limit,
            bandwidth_limit_gb,
            "updated quota limits"
        );
        Ok(())
    }

    pub fn reset_quota(&self, tenant_id: &str) -> Result<(), QuotaError> {
        self.ensure_entry(tenant_id);

        if let Some(mut metrics) = self.cache.get_mut(tenant_id) {
            metrics.message_count = 0;
            metrics.bytes_sent = 0;
            metrics.period = current_day_period();
            metrics.last_reset = Utc::now();
        }

        self.database
            .save_usage(tenant_id, &current_day_period(), MESSAGE_QUOTA_TYPE, 0)?;
        self.database
            .save_usage(tenant_id, &current_month_period(), BANDWIDTH_QUOTA_TYPE, 0)?;

        Ok(())
    }

    pub fn persist_all(&self) -> Result<usize, StorageError> {
        let mut persisted = 0usize;
        let month_period = current_month_period();

        for entry in self.cache.iter() {
            let tenant_id = entry.key().clone();
            let metrics = entry.value().clone();
            self.database.save_usage(
                &tenant_id,
                &metrics.period,
                MESSAGE_QUOTA_TYPE,
                metrics.message_count,
            )?;
            self.database.save_usage(
                &tenant_id,
                &month_period,
                BANDWIDTH_QUOTA_TYPE,
                metrics.bytes_sent,
            )?;
            persisted += 1;
        }

        Ok(persisted)
    }

    pub fn start_persistence_task(&self) -> JoinHandle<()> {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(manager.persistence_interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                match manager.persist_all() {
                    Ok(count) if count > 0 => {
                        debug!(persisted = count, "persisted quota usage");
                    }
                    Ok(_) => {
                        debug!("no quota metrics to persist");
                    }
                    Err(err) => {
                        error!(error = %err, "failed to persist quota usage");
                    }
                }
            }
        })
    }

    fn ensure_entry(&self, tenant_id: &str) {
        if self.cache.contains_key(tenant_id) {
            return;
        }

        let day_period = current_day_period();
        let bytes_limit = (self.default_bandwidth_limit_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        let limits = self.database.get_quota_limits(tenant_id).ok().flatten();

        let (message_limit, bandwidth_limit_bytes) = match limits {
            Some(limit) => (limit.message_limit, limit.bandwidth_limit_bytes),
            None => {
                if let Err(err) = self.database.set_quota_limits(
                    tenant_id,
                    self.default_message_limit,
                    self.default_bandwidth_limit_gb,
                ) {
                    error!(
                        tenant_id,
                        error = %err,
                        "failed to initialize quota limits from defaults"
                    );
                }
                (self.default_message_limit, bytes_limit)
            }
        };

        let metrics = QuotaMetrics {
            tenant_id: tenant_id.to_string(),
            message_count: 0,
            bytes_sent: 0,
            message_limit,
            bandwidth_limit_bytes,
            last_reset: Utc::now(),
            period: day_period,
        };

        self.cache.insert(tenant_id.to_string(), metrics);
    }
}

fn current_day_period() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

fn current_month_period() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}", now.year(), now.month())
}
