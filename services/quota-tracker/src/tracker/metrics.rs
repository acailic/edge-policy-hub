use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaMetrics {
    pub tenant_id: String,
    pub message_count: u64,
    pub bytes_sent: u64,
    pub message_limit: u64,
    pub bandwidth_limit_bytes: u64,
    pub last_reset: DateTime<Utc>,
    pub period: String,
}

impl Default for QuotaMetrics {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            message_count: 0,
            bytes_sent: 0,
            message_limit: 0,
            bandwidth_limit_bytes: 0,
            last_reset: Utc::now(),
            period: String::new(),
        }
    }
}

impl QuotaMetrics {
    pub fn message_percentage(&self) -> f64 {
        if self.message_limit == 0 {
            return 0.0;
        }
        (self.message_count as f64 / self.message_limit as f64) * 100.0
    }

    pub fn bandwidth_percentage(&self) -> f64 {
        if self.bandwidth_limit_bytes == 0 {
            return 0.0;
        }
        (self.bytes_sent as f64 / self.bandwidth_limit_bytes as f64) * 100.0
    }

    pub fn is_message_limit_exceeded(&self) -> bool {
        self.message_limit > 0 && self.message_count >= self.message_limit
    }

    pub fn is_bandwidth_limit_exceeded(&self) -> bool {
        self.bandwidth_limit_bytes > 0 && self.bytes_sent >= self.bandwidth_limit_bytes
    }

    pub fn remaining_messages(&self) -> u64 {
        self.message_limit.saturating_sub(self.message_count)
    }

    pub fn remaining_bandwidth_gb(&self) -> f64 {
        if self.bandwidth_limit_bytes == 0 {
            0.0
        } else {
            let remaining = self
                .bandwidth_limit_bytes
                .saturating_sub(self.bytes_sent) as f64;
            remaining / (1024.0 * 1024.0 * 1024.0)
        }
    }
}
