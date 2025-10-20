use std::io;

use rusqlite;
use serde_json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("tenant {0} not found")]
    TenantNotFound(String),
    #[error("invalid log entry: {0}")]
    InvalidLogEntry(String),
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
