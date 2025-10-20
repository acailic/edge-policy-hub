use std::sync::Arc;

use dashmap::DashMap;
use tracing::debug;

use crate::auth::TenantContext;

pub struct SessionStore {
    sessions: Arc<DashMap<String, TenantContext>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn store_context(&self, client_id: String, context: TenantContext) {
        debug!(
            "Storing session context for client '{}' with tenant '{}'",
            client_id, context.tenant_id
        );
        self.sessions.insert(client_id, context);
    }

    pub fn get_context(&self, client_id: &str) -> Option<TenantContext> {
        self.sessions.get(client_id).map(|entry| entry.clone())
    }

    pub fn remove_context(&self, client_id: &str) -> Option<TenantContext> {
        debug!("Removing session context for client '{}'", client_id);
        self.sessions.remove(client_id).map(|(_, ctx)| ctx)
    }

    pub fn list_tenants(&self) -> Vec<String> {
        let mut tenants: Vec<String> = self
            .sessions
            .iter()
            .map(|entry| entry.value().tenant_id.clone())
            .collect();

        tenants.sort();
        tenants.dedup();
        tenants
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
