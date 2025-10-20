use std::net::IpAddr;

use super::AuthSource;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub client_id: String,
    pub auth_source: AuthSource,
    pub client_ip: Option<IpAddr>,
    pub connection_id: String,
}

impl TenantContext {
    pub fn new(tenant_id: String, client_id: String, auth_source: AuthSource) -> Self {
        Self {
            tenant_id,
            user_id: None,
            device_id: None,
            client_id,
            auth_source,
            client_ip: None,
            connection_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_device_id(mut self, device_id: String) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_client_ip(mut self, ip: IpAddr) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn with_connection_id(mut self, id: String) -> Self {
        self.connection_id = id;
        self
    }
}
