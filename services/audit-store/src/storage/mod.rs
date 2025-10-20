pub mod database;
pub mod error;
pub mod policy_bundles;
pub mod schema;
pub mod tenant_registry;

pub use database::AuditDatabase;
pub use error::StorageError;
pub use policy_bundles::PolicyBundleStore;
pub use tenant_registry::TenantRegistry;

pub const AUDIT_DB_FILENAME: &str = "audit.db";
pub const TENANT_DB_FILENAME: &str = "tenants.db";
pub const BUNDLES_DB_FILENAME: &str = "policy_bundles.db";
