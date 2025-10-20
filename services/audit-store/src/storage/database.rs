use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use dashmap::DashMap;
use rusqlite::{params, Connection};
use tracing::{debug, info};

use crate::api::types::AuditLogEntry;

use super::error::StorageError;
use super::{schema::init_database, AUDIT_DB_FILENAME};

#[derive(Clone, Debug, Default)]
pub struct LogFilter {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub decision: Option<String>,
    pub protocol: Option<String>,
    pub limit: Option<usize>,
}

pub struct AuditDatabase {
    data_dir: PathBuf,
    connections: DashMap<String, Arc<Mutex<Connection>>>,
}

impl AuditDatabase {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            data_dir,
            connections: DashMap::new(),
        })
    }

    fn tenant_dir(&self, tenant_id: &str) -> PathBuf {
        self.data_dir.join(tenant_id)
    }

    fn get_or_create_connection(
        &self,
        tenant_id: &str,
    ) -> Result<Arc<Mutex<Connection>>, StorageError> {
        if let Some(entry) = self.connections.get(tenant_id) {
            return Ok(entry.clone());
        }

        let tenant_dir = self.tenant_dir(tenant_id);
        std::fs::create_dir_all(&tenant_dir)?;
        let db_path = tenant_dir.join(AUDIT_DB_FILENAME);
        let is_new = !db_path.exists();
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        if is_new {
            init_database(&conn)?;
            info!(tenant_id, "initialized audit database");
        }

        let conn = Arc::new(Mutex::new(conn));
        self.connections
            .insert(tenant_id.to_string(), Arc::clone(&conn));
        Ok(conn)
    }

    pub fn write_audit_log(
        &self,
        tenant_id: &str,
        log: &AuditLogEntry,
    ) -> Result<(), StorageError> {
        let conn = self.get_or_create_connection(tenant_id)?;
        let conn = conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let subject = serde_json::to_string(&log.subject)?;
        let resource = serde_json::to_string(&log.resource)?;
        let environment = serde_json::to_string(&log.environment)?;

        conn.execute(
            r#"
            INSERT INTO audit_logs (
                log_id,
                tenant_id,
                timestamp,
                decision,
                protocol,
                subject,
                action,
                resource,
                environment,
                policy_version,
                reason,
                signature,
                uploaded
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                log.log_id,
                log.tenant_id,
                log.timestamp,
                log.decision,
                log.protocol,
                subject,
                log.action,
                resource,
                environment,
                log.policy_version.map(|v| v as i64),
                log.reason,
                log.signature,
                if log.uploaded { 1 } else { 0 },
            ],
        )?;

        debug!(tenant_id, log_id = %log.log_id, "stored audit log entry");
        Ok(())
    }

    pub fn query_logs(
        &self,
        tenant_id: &str,
        filter: &LogFilter,
    ) -> Result<Vec<AuditLogEntry>, StorageError> {
        let conn = self.get_or_create_connection(tenant_id)?;
        let conn = conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;

        let mut conditions = vec!["tenant_id = :tenant_id".to_string()];
        let mut bindings: Vec<(String, rusqlite::types::Value)> =
            vec![(":tenant_id".into(), tenant_id.into())];

        if let Some(start) = &filter.start_time {
            conditions.push("timestamp >= :start_time".into());
            bindings.push((":start_time".into(), start.clone().into()));
        }
        if let Some(end) = &filter.end_time {
            conditions.push("timestamp <= :end_time".into());
            bindings.push((":end_time".into(), end.clone().into()));
        }
        if let Some(decision) = &filter.decision {
            conditions.push("decision = :decision".into());
            bindings.push((":decision".into(), decision.clone().into()));
        }
        if let Some(protocol) = &filter.protocol {
            conditions.push("protocol = :protocol".into());
            bindings.push((":protocol".into(), protocol.clone().into()));
        }

        let mut sql = format!(
            "SELECT log_id, tenant_id, timestamp, decision, protocol, subject, action, resource, environment, policy_version, reason, signature, uploaded FROM audit_logs WHERE {} ORDER BY timestamp DESC",
            conditions.join(" AND ")
        );

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&limit.to_string());
        }

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map_named(
            bindings
                .iter()
                .map(|(k, v)| (k.as_str(), v))
                .collect::<Vec<_>>()
                .as_slice(),
            |row| {
                let subject: String = row.get(5)?;
                let resource: String = row.get(7)?;
                let environment: String = row.get(8)?;

                Ok(AuditLogEntry {
                    log_id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    decision: row.get(3)?,
                    protocol: row.get(4)?,
                    subject: serde_json::from_str(&subject)?,
                    action: row.get(6)?,
                    resource: serde_json::from_str(&resource)?,
                    environment: serde_json::from_str(&environment)?,
                    policy_version: row
                        .get::<_, Option<i64>>(9)?
                        .and_then(|value| {
                            if value < 0 {
                                None
                            } else {
                                u32::try_from(value).ok()
                            }
                        }),
                    reason: row.get(10)?,
                    signature: row.get(11)?,
                    uploaded: row.get::<_, i64>(12)? != 0,
                })
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    pub fn get_unuploaded_logs(
        &self,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<AuditLogEntry>, StorageError> {
        let conn = self.get_or_create_connection(tenant_id)?;
        let conn = conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT log_id, tenant_id, timestamp, decision, protocol, subject, action, resource, environment, policy_version, reason, signature, uploaded
            FROM audit_logs
            WHERE tenant_id = ?1 AND uploaded = 0
            ORDER BY timestamp ASC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![tenant_id, limit as i64], |row| {
            let subject: String = row.get(5)?;
            let resource: String = row.get(7)?;
            let environment: String = row.get(8)?;

            Ok(AuditLogEntry {
                log_id: row.get(0)?,
                tenant_id: row.get(1)?,
                timestamp: row.get(2)?,
                decision: row.get(3)?,
                protocol: row.get(4)?,
                subject: serde_json::from_str(&subject)?,
                action: row.get(6)?,
                resource: serde_json::from_str(&resource)?,
                environment: serde_json::from_str(&environment)?,
                policy_version: row
                    .get::<_, Option<i64>>(9)?
                    .and_then(|value| {
                        if value < 0 {
                            None
                        } else {
                            u32::try_from(value).ok()
                        }
                    }),
                reason: row.get(10)?,
                signature: row.get(11)?,
                uploaded: row.get::<_, i64>(12)? != 0,
            })
        })?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        Ok(logs)
    }

    pub fn mark_logs_uploaded(
        &self,
        tenant_id: &str,
        log_ids: &[String],
    ) -> Result<(), StorageError> {
        if log_ids.is_empty() {
            return Ok(());
        }

        let conn = self.get_or_create_connection(tenant_id)?;
        let mut conn = conn
            .lock()
            .map_err(|_| StorageError::InvalidLogEntry("connection poisoned".into()))?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                UPDATE audit_logs
                SET uploaded = 1
                WHERE log_id = ?1
                "#,
            )?;

            for log_id in log_ids {
                stmt.execute(params![log_id])?;
            }
        }

        tx.commit()?;
        Ok(())
    }
}
