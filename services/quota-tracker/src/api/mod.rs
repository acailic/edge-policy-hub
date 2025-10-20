use std::sync::Arc;

pub mod handlers;
pub mod router;
pub mod types;

pub use handlers::*;
pub use router::create_router;
pub use types::*;

use crate::config::QuotaTrackerConfig;
use crate::tracker::QuotaManager;

pub struct ApiState {
    pub quota_manager: Arc<QuotaManager>,
    pub config: Arc<QuotaTrackerConfig>,
}

impl ApiState {
    pub fn new(quota_manager: Arc<QuotaManager>, config: QuotaTrackerConfig) -> Self {
        Self {
            quota_manager,
            config: Arc::new(config),
        }
    }
}
