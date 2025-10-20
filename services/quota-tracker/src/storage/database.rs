use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::error::StorageError;
use super::schema::init_database;
use super::QUOTA_DB_FILENAME;

#[derive(Debug, Clone)]
pub struct QuotaLimits {
    pub tenant_id: String,
    pub message_limit: u64,
    pub bandwidth_limit_bytes: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct QuotaUsageRecord {
    pub tenant_id: String,
    pub period: String,
    pub quota_type: String,
    pub used: u64,
    pub last_updated: String,
}

pub struct QuotaDatabase {
    data_dir: PathBuf,
    conn: Mutex<Connection>,
}

impl QuotaDatabase {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join(QUOTA_DB_FILENAME);
        let is_new = !db_path.exists();
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        if is_new {
            init_database(&conn)?;
        }

        Ok(Self {
            data_dir,
            conn: Mutex::new(conn),
        })
    }

    pub fn set_quota_limits(
        &self,
        tenant_id: &str,
        message_limit: u64,
        bandwidth_limit_gb: f64,
    ) -> Result<(), StorageError> {
        if message_limit == 0 {
            return Err(StorageError::InvalidQuotaValue(
                "message limit must be greater than zero".into(),
            ));
        }
        if bandwidth_limit_gb <= 0.0 {
            return Err(StorageError::InvalidQuotaValue(
                "bandwidth limit must be greater than zero".into(),
            ));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;

        let now = Utc::now().to_rfc3339();
        let bytes_limit = (bandwidth_limit_gb * 1024.0 * 1024.0 * 1024.0) as u64;

        conn.execute(
            r#"
            INSERT INTO quota_limits (tenant_id, message_limit, bandwidth_limit_bytes, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(tenant_id) DO UPDATE SET
                message_limit = excluded.message_limit,
                bandwidth_limit_bytes = excluded.bandwidth_limit_bytes,
                updated_at = excluded.updated_at
            "#,
            params![tenant_id, message_limit as i64, bytes_limit as i64, now, now],
        )?;

        Ok(())
    }

    pub fn get_quota_limits(
        &self,
        tenant_id: &str,
    ) -> Result<Option<QuotaLimits>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT tenant_id, message_limit, bandwidth_limit_bytes, created_at, updated_at
            FROM quota_limits
            WHERE tenant_id = ?1
            "#,
        )?;

        let result = stmt
            .query_row(params![tenant_id], |row| {
                Ok(QuotaLimits {
                    tenant_id: row.get(0)?,
                    message_limit: row.get::<_, i64>(1)? as u64,
                    bandwidth_limit_bytes: row.get::<_, i64>(2)? as u64,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn save_usage(
        &self,
        tenant_id: &str,
        period: &str,
        quota_type: &str,
        used: u64,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO quota_usage (tenant_id, period, quota_type, used, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(tenant_id, period, quota_type) DO UPDATE SET
                used = excluded.used,
                last_updated = excluded.last_updated
            "#,
            params![tenant_id, period, quota_type, used as i64, now],
        )?;

        Ok(())
    }

    pub fn load_usage(
        &self,
        tenant_id: &str,
        period: &str,
        quota_type: &str,
    ) -> Result<u64, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT used
            FROM quota_usage
            WHERE tenant_id = ?1 AND period = ?2 AND quota_type = ?3
            "#,
        )?;

        let used = stmt
            .query_row(params![tenant_id, period, quota_type], |row| row.get::<_, i64>(0))
            .optional()?;

        Ok(used.unwrap_or(0) as u64)
    }

    pub fn list_tenant_usage(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<QuotaUsageRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT tenant_id, period, quota_type, used, last_updated
            FROM quota_usage
            WHERE tenant_id = ?1
            ORDER BY period DESC
            "#,
        )?;

        let rows = stmt.query_map(params![tenant_id], |row| {
            Ok(QuotaUsageRecord {
                tenant_id: row.get(0)?,
                period: row.get(1)?,
                quota_type: row.get(2)?,
                used: row.get::<_, i64>(3)? as u64,
                last_updated: row.get(4)?,
            })
        })?;

        let mut usage = Vec::new();
        for row in rows {
            usage.push(row?);
        }
        Ok(usage)
    }

    pub fn list_limits(&self) -> Result<Vec<QuotaLimits>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidQuotaValue("connection poisoned".into()))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT tenant_id, message_limit, bandwidth_limit_bytes, created_at, updated_at
            FROM quota_limits
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(QuotaLimits {
                tenant_id: row.get(0)?,
                message_limit: row.get::<_, i64>(1)? as u64,
                bandwidth_limit_bytes: row.get::<_, i64>(2)? as u64,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut limits = Vec::new();
        for row in rows {
            limits.push(row?);
        }
        Ok(limits)
    }
}
