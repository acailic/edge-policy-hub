mod error;
mod transformer;

pub use error::TransformError;
pub use transformer::PayloadTransformer;

#[derive(Debug, Clone)]
pub enum TransformDirective {
    RemoveFields(Vec<String>),
    RedactFields(Vec<String>),
    StripCoordinates,
}

pub const MAX_TRANSFORM_DEPTH: usize = 10;
pub const REDACTED_PLACEHOLDER: &str = "[REDACTED]";
