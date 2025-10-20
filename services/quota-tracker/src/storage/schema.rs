use anyhow::Result;
use rusqlite::Connection;

pub const QUOTA_LIMITS_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS quota_limits (
    tenant_id TEXT PRIMARY KEY,
    message_limit INTEGER NOT NULL,
    bandwidth_limit_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

pub const QUOTA_USAGE_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS quota_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    period TEXT NOT NULL,
    quota_type TEXT NOT NULL,
    used INTEGER NOT NULL,
    last_updated TEXT NOT NULL,
    UNIQUE(tenant_id, period, quota_type)
);
"#;

pub const QUOTA_USAGE_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_usage_tenant_period ON quota_usage(tenant_id, period);
"#;

pub fn init_database(conn: &Connection) -> Result<()> {
    conn.execute_batch(QUOTA_LIMITS_TABLE_SCHEMA)?;
    conn.execute_batch(QUOTA_USAGE_TABLE_SCHEMA)?;
    conn.execute_batch(QUOTA_USAGE_INDEXES)?;
    Ok(())
}
