use std::io;

use rusqlite;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("tenant {0} not found")]
    TenantNotFound(String),
    #[error("invalid quota value: {0}")]
    InvalidQuotaValue(String),
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
}
