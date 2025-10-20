mod context;
mod error;
mod extractor;

pub use context::TenantContext;
pub use error::AuthError;
pub use extractor::TenantExtractor;

#[derive(Debug, Clone)]
pub enum AuthSource {
    Certificate,
    Username,
    ClientId,
}

pub const USERNAME_SEPARATOR: char = ':';
pub const CLIENTID_SEPARATOR: char = '/';
