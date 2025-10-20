use std::sync::Arc;

use anyhow::Result;

pub mod handlers;
pub mod router;
pub mod types;

pub use handlers::*;
pub use router::create_router;
pub use types::*;

use crate::config::AuditStoreConfig;
use crate::signing::Signer;
use crate::storage::{AuditDatabase, PolicyBundleStore, TenantRegistry};

pub struct ApiState {
    pub database: Arc<AuditDatabase>,
    pub tenant_registry: Arc<TenantRegistry>,
    pub bundle_store: Arc<PolicyBundleStore>,
    pub signer: Arc<Signer>,
    pub config: Arc<AuditStoreConfig>,
}

impl ApiState {
    pub fn new(config: AuditStoreConfig) -> Result<Self> {
        let data_dir = config.data_dir.clone();
        let database = Arc::new(AuditDatabase::new(data_dir.clone())?);
        let tenant_registry = Arc::new(TenantRegistry::new(&data_dir)?);
        let bundle_store = Arc::new(PolicyBundleStore::new(&data_dir)?);
        let signer = Arc::new(Signer::new(&config.hmac_secret_key)?);

        Ok(Self {
            database,
            tenant_registry,
            bundle_store,
            signer,
            config: Arc::new(config),
        })
    }
}
