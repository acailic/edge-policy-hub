use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::error::StorageError;
use super::schema::POLICY_BUNDLES_TABLE_SCHEMA;
use super::BUNDLES_DB_FILENAME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundleRecord {
    pub bundle_id: String,
    pub tenant_id: String,
    pub version: i64,
    pub rego_code: String,
    pub metadata: Option<serde_json::Value>,
    pub status: String,
    pub created_at: String,
    pub activated_at: Option<String>,
}

pub struct PolicyBundleStore {
    conn: Mutex<Connection>,
}

impl PolicyBundleStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join(BUNDLES_DB_FILENAME);
        let is_new = !db_path.exists();
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        if is_new {
            conn.execute_batch(POLICY_BUNDLES_TABLE_SCHEMA)?;
        }

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn store_bundle(&self, bundle: &PolicyBundleRecord) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let next_version = query_next_version(&conn, &bundle.tenant_id)?;
        let metadata = match &bundle.metadata {
            Some(value) => Some(serde_json::to_string(value)?),
            None => None,
        };

        conn.execute(
            r#"
            INSERT INTO policy_bundles (
                bundle_id,
                tenant_id,
                version,
                rego_code,
                metadata,
                status,
                created_at,
                activated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                bundle.bundle_id,
                bundle.tenant_id,
                next_version,
                bundle.rego_code,
                metadata,
                bundle.status,
                bundle.created_at,
                bundle.activated_at,
            ],
        )?;

        info!(
            tenant_id = %bundle.tenant_id,
            bundle_id = %bundle.bundle_id,
            version = next_version,
            "stored policy bundle"
        );

        Ok(())
    }

    pub fn get_bundle(
        &self,
        bundle_id: &str,
    ) -> Result<Option<PolicyBundleRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT bundle_id, tenant_id, version, rego_code, metadata, status, created_at, activated_at
            FROM policy_bundles
            WHERE bundle_id = ?1
            "#,
        )?;

        let row = stmt
            .query_row(params![bundle_id], |row| {
                let metadata: Option<String> = row.get(4)?;
                Ok(PolicyBundleRecord {
                    bundle_id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    version: row.get(2)?,
                    rego_code: row.get(3)?,
                    metadata: metadata
                        .map(|value| serde_json::from_str(&value))
                        .transpose()?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    activated_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn get_active_bundle(
        &self,
        tenant_id: &str,
    ) -> Result<Option<PolicyBundleRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT bundle_id, tenant_id, version, rego_code, metadata, status, created_at, activated_at
            FROM policy_bundles
            WHERE tenant_id = ?1 AND status = 'active'
            ORDER BY version DESC
            LIMIT 1
            "#,
        )?;

        let row = stmt
            .query_row(params![tenant_id], |row| {
                let metadata: Option<String> = row.get(4)?;
                Ok(PolicyBundleRecord {
                    bundle_id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    version: row.get(2)?,
                    rego_code: row.get(3)?,
                    metadata: metadata
                        .map(|value| serde_json::from_str(&value))
                        .transpose()?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    activated_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(row)
    }

    pub fn list_bundles(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<PolicyBundleRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT bundle_id, tenant_id, version, rego_code, metadata, status, created_at, activated_at
            FROM policy_bundles
            WHERE tenant_id = ?1
            ORDER BY version DESC
            "#,
        )?;

        let rows = stmt.query_map(params![tenant_id], |row| {
            let metadata: Option<String> = row.get(4)?;
            Ok(PolicyBundleRecord {
                bundle_id: row.get(0)?,
                tenant_id: row.get(1)?,
                version: row.get(2)?,
                rego_code: row.get(3)?,
                metadata: metadata
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                activated_at: row.get(7)?,
            })
        })?;

        let mut bundles = Vec::new();
        for row in rows {
            bundles.push(row?);
        }
        Ok(bundles)
    }

    pub fn activate_bundle(&self, bundle_id: &str) -> Result<(), StorageError> {
        let bundle = self
            .get_bundle(bundle_id)?
            .ok_or_else(|| StorageError::InvalidLogEntry("bundle not found".into()))?;
        let tenant_id = bundle.tenant_id.clone();

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            UPDATE policy_bundles
            SET status = 'inactive', activated_at = NULL
            WHERE tenant_id = ?1
            "#,
            params![tenant_id],
        )?;

        tx.execute(
            r#"
            UPDATE policy_bundles
            SET status = 'active', activated_at = ?2
            WHERE bundle_id = ?1
            "#,
            params![bundle_id, Utc::now().to_rfc3339()],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn archive_bundle(&self, bundle_id: &str) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let updated = conn.execute(
            r#"
            UPDATE policy_bundles
            SET status = 'archived'
            WHERE bundle_id = ?1
            "#,
            params![bundle_id],
        )?;

        if updated == 0 {
            return Err(StorageError::InvalidLogEntry("bundle not found".into()));
        }

        Ok(())
    }

}

fn query_next_version(conn: &Connection, tenant_id: &str) -> Result<i64, StorageError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT MAX(version)
        FROM policy_bundles
        WHERE tenant_id = ?1
        "#,
    )?;

    let current_version: Option<i64> = stmt
        .query_row(params![tenant_id], |row| row.get(0))
        .optional()?;
    Ok(current_version.unwrap_or(0) + 1)
}
