use anyhow::Result;
use rusqlite::Connection;

pub const TENANTS_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tenants (
    tenant_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    config TEXT
);
"#;

pub const POLICY_BUNDLES_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS policy_bundles (
    bundle_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    rego_code TEXT NOT NULL,
    metadata TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    activated_at TEXT,
    UNIQUE(tenant_id, version)
);
"#;

pub const AUDIT_LOGS_TABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audit_logs (
    log_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    decision TEXT NOT NULL,
    protocol TEXT NOT NULL,
    subject TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    environment TEXT NOT NULL,
    policy_version INTEGER,
    reason TEXT,
    signature TEXT NOT NULL,
    uploaded INTEGER DEFAULT 0
);
"#;

pub const AUDIT_LOGS_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_audit_tenant_timestamp ON audit_logs(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_uploaded ON audit_logs(uploaded);
"#;

pub fn init_database(conn: &Connection) -> Result<()> {
    conn.execute_batch(TENANTS_TABLE_SCHEMA)?;
    conn.execute_batch(POLICY_BUNDLES_TABLE_SCHEMA)?;
    conn.execute_batch(AUDIT_LOGS_TABLE_SCHEMA)?;
    conn.execute_batch(AUDIT_LOGS_INDEXES)?;
    Ok(())
}
