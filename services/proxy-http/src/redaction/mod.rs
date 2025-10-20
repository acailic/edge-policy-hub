mod engine;
mod error;

pub use engine::RedactionEngine;
pub use error::RedactionError;

pub type RedactionPath = String;

pub const MAX_REDACTION_DEPTH: usize = 10;
pub const REDACTED_PLACEHOLDER: &str = "[REDACTED]";
