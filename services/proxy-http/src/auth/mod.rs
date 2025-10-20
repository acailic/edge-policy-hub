mod context;
mod error;
mod extractor;

pub use context::TenantContext;
pub use error::AuthError;
pub use extractor::TenantExtractor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    MTls,
    Jwt,
    Header,
}

pub const TENANT_ID_HEADER: &str = "X-Tenant-ID";
pub const AUTHORIZATION_HEADER: &str = "Authorization";
