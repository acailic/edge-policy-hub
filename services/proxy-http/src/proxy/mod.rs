mod error;
pub(crate) mod handler;
mod upstream;

pub use error::ProxyError;
pub use handler::ProxyHandler;
pub use upstream::UpstreamClient;

use crate::auth::TenantExtractor;
use crate::config::ProxyConfig;
use crate::policy::PolicyClient;
use crate::quota::QuotaClient;
use crate::redaction::RedactionEngine;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<ProxyConfig>,
    pub tenant_extractor: Arc<TenantExtractor>,
    pub policy_client: Arc<PolicyClient>,
    pub redaction_engine: Arc<RedactionEngine>,
    pub upstream_client: Arc<UpstreamClient>,
    pub quota_client: Option<Arc<QuotaClient>>,
}

impl ProxyState {
    pub fn new(config: ProxyConfig) -> anyhow::Result<Self> {
        use anyhow::Context;

        let tenant_extractor = Arc::new(TenantExtractor::new(&config)?);
        let policy_client = Arc::new(PolicyClient::new(
            config.enforcer_url.clone(),
            crate::policy::DEFAULT_ENFORCER_TIMEOUT_SECS,
        )?);
        let redaction_engine = Arc::new(RedactionEngine::new());
        let upstream_client = Arc::new(UpstreamClient::new(
            config.upstream_url.clone(),
            config.request_timeout_secs,
            config.max_body_size_bytes,
            config.forward_auth_header,
        )?);
        let quota_client = if let Some(url) = config.quota_tracker_url.clone() {
            let token = config
                .quota_tracker_token
                .clone()
                .context("Quota tracker token missing despite validation")?;
            Some(Arc::new(QuotaClient::new(url, token)?))
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            tenant_extractor,
            policy_client,
            redaction_engine,
            upstream_client,
            quota_client,
        })
    }
}
