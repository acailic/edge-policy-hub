pub mod database;
pub mod error;
pub mod schema;

pub use database::{QuotaDatabase, QuotaLimits, QuotaUsageRecord};
pub use error::StorageError;

pub const QUOTA_DB_FILENAME: &str = "quotas.db";
pub const QUOTA_LIMITS_TABLE: &str = "quota_limits";
pub const QUOTA_USAGE_TABLE: &str = "quota_usage";
