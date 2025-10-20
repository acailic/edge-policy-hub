use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::schema::TENANTS_TABLE_SCHEMA;
use super::TENANT_DB_FILENAME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRecord {
    pub tenant_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub config: Option<serde_json::Value>,
}

pub struct TenantRegistry {
    conn: Mutex<Connection>,
}

impl TenantRegistry {
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join(TENANT_DB_FILENAME);
        let is_new = !db_path.exists();
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        if is_new {
            conn.execute_batch(TENANTS_TABLE_SCHEMA)?;
        }

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_tenant(&self, tenant: &TenantRecord) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let config = tenant.config.as_ref().map(|cfg| serde_json::to_string(cfg));
        let config = match config {
            Some(Ok(value)) => Some(value),
            Some(Err(err)) => return Err(StorageError::SerializationError(err)),
            None => None,
        };

        conn.execute(
            r#"
            INSERT INTO tenants (tenant_id, name, status, created_at, updated_at, config)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                tenant.tenant_id,
                tenant.name,
                tenant.status,
                tenant.created_at,
                tenant.updated_at,
                config
            ],
        )?;
        Ok(())
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Result<Option<TenantRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT tenant_id, name, status, created_at, updated_at, config
            FROM tenants
            WHERE tenant_id = ?1
            "#,
        )?;

        let row = stmt
            .query_row(params![tenant_id], |row| {
                let config: Option<String> = row.get(5)?;
                Ok(TenantRecord {
                    tenant_id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    config: match config {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    },
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn update_tenant(&self, tenant: &TenantRecord) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let config = tenant.config.as_ref().map(|cfg| serde_json::to_string(cfg));
        let config = match config {
            Some(Ok(value)) => Some(value),
            Some(Err(err)) => return Err(StorageError::SerializationError(err)),
            None => None,
        };

        let updated = conn.execute(
            r#"
            UPDATE tenants
            SET name = ?2,
                status = ?3,
                updated_at = ?4,
                config = ?5
            WHERE tenant_id = ?1
            "#,
            params![
                tenant.tenant_id,
                tenant.name,
                tenant.status,
                tenant.updated_at,
                config
            ],
        )?;

        if updated == 0 {
            return Err(StorageError::TenantNotFound(tenant.tenant_id.clone()));
        }

        Ok(())
    }

    pub fn list_tenants(
        &self,
        status_filter: Option<&str>,
    ) -> Result<Vec<TenantRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let mut sql = String::from(
            r#"
            SELECT tenant_id, name, status, created_at, updated_at, config
            FROM tenants
            "#,
        );
        let mut params_vec: Vec<String> = Vec::new();

        if let Some(status) = status_filter {
            sql.push_str("WHERE status = ?1");
            params_vec.push(status.to_string());
        }

        sql.push_str(" ORDER BY created_at DESC");
        let mut stmt = conn.prepare(&sql)?;

        let rows = if params_vec.is_empty() {
            stmt.query_map([], |row| {
                let config: Option<String> = row.get(5)?;
                Ok(TenantRecord {
                    tenant_id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    config: match config {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    },
                })
            })?
        } else {
            stmt.query_map([params_vec[0].clone()], |row| {
                let config: Option<String> = row.get(5)?;
                Ok(TenantRecord {
                    tenant_id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    config: match config {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    },
                })
            })?
        };

        let mut tenants = Vec::new();
        for row in rows {
            tenants.push(row?);
        }
        Ok(tenants)
    }

    pub fn delete_tenant(&self, tenant_id: &str) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let now = Utc::now().to_rfc3339();
        let updated = conn.execute(
            r#"
            UPDATE tenants
            SET status = 'deleted',
                updated_at = ?2
            WHERE tenant_id = ?1
            "#,
            params![tenant_id, now],
        )?;

        if updated == 0 {
            return Err(StorageError::TenantNotFound(tenant_id.to_string()));
        }

        Ok(())
    }
}
