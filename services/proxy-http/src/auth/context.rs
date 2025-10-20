use super::AuthMethod;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub roles: Vec<String>,
    pub auth_method: AuthMethod,
    pub client_ip: Option<IpAddr>,
    pub request_id: String,
}

impl TenantContext {
    pub fn new(tenant_id: String, auth_method: AuthMethod) -> Self {
        Self {
            tenant_id,
            user_id: None,
            device_id: None,
            roles: Vec::new(),
            auth_method,
            client_ip: None,
            request_id: uuid::Uuid::new_v4().to_string(),
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

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_client_ip(mut self, ip: IpAddr) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn with_request_id(mut self, id: String) -> Self {
        self.request_id = id;
        self
    }
}
