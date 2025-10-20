mod error;
mod tracker;

pub use error::QuotaError;
pub use tracker::{QuotaMetrics, QuotaTracker};

pub const DEFAULT_MESSAGE_LIMIT: u64 = 50_000;
pub const DEFAULT_BANDWIDTH_LIMIT_GB: f64 = 100.0;
