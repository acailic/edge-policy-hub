mod handler;
mod session;

pub use handler::PolicyHookHandler;
pub use session::SessionStore;

use std::sync::Arc;

use anyhow::Result;

use crate::{
    auth::TenantExtractor, config::BridgeConfig, policy::PolicyClient,
    quota::QuotaTracker, transform::PayloadTransformer,
};

#[derive(Clone)]
pub struct HookContext {
    pub tenant_extractor: Arc<TenantExtractor>,
    pub policy_client: Arc<PolicyClient>,
    pub payload_transformer: Arc<PayloadTransformer>,
    pub quota_tracker: Arc<QuotaTracker>,
    pub session_store: Arc<SessionStore>,
    pub config: Arc<BridgeConfig>,
}

impl HookContext {
    pub fn new(config: BridgeConfig) -> Result<Self> {
        let tenant_extractor = Arc::new(TenantExtractor::new(&config));
        let policy_client = Arc::new(PolicyClient::new(
            config.enforcer_url.clone(),
            config.request_timeout_secs,
            config.use_mqtt_endpoints,
        )?);
        let payload_transformer = Arc::new(PayloadTransformer::new());
        let quota_tracker = Arc::new(QuotaTracker::new(
            config.message_limit,
            config.bandwidth_limit_gb,
        ));
        let session_store = Arc::new(SessionStore::new());

        Ok(Self {
            tenant_extractor,
            policy_client,
            payload_transformer,
            quota_tracker,
            session_store,
            config: Arc::new(config),
        })
    }
}
