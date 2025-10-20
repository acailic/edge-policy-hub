use std::net::IpAddr;

use tracing::{debug, warn};
use x509_parser::prelude::*;

use crate::config::BridgeConfig;

use super::{AuthError, AuthSource, TenantContext, CLIENTID_SEPARATOR, USERNAME_SEPARATOR};

pub struct TenantExtractor {
    enable_mtls: bool,
    cert_cn_as_username: bool,
    topic_namespace_pattern: String,
}

impl TenantExtractor {
    pub fn new(config: &BridgeConfig) -> Self {
        Self {
            enable_mtls: config.enable_mtls,
            cert_cn_as_username: config.cert_cn_as_username,
            topic_namespace_pattern: config.topic_namespace_pattern.clone(),
        }
    }

    pub fn extract_from_certificate(&self, cert_der: &[u8]) -> Result<String, AuthError> {
        let (_, cert) = parse_x509_certificate(cert_der)?;

        // Try extracting from Subject Alternative Name (SAN) with URI format tenant:{id}
        if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
            for name in &san_ext.value.general_names {
                if let GeneralName::URI(uri) = name {
                    if uri.starts_with("tenant:") {
                        let tenant_id = uri.strip_prefix("tenant:").unwrap().to_string();
                        debug!("Extracted tenant_id from certificate SAN URI: {}", tenant_id);
                        return Ok(tenant_id);
                    }
                }
            }
        }

        // Fallback to Common Name (CN)
        if let Some(cn) = cert.subject().iter_common_name().next() {
            if let Ok(cn_str) = cn.as_str() {
                let tenant_id = cn_str.to_string();
                debug!("Extracted tenant_id from certificate CN: {}", tenant_id);
                return Ok(tenant_id);
            }
        }

        Err(AuthError::TenantIdNotFound)
    }

    pub fn extract_from_username(
        &self,
        username: &str,
    ) -> Result<(String, Option<String>), AuthError> {
        if username.is_empty() {
            return Err(AuthError::InvalidUsernameFormat(
                "Username is empty".to_string(),
            ));
        }

        // Parse username format: tenant_id:user_id or just tenant_id
        let parts: Vec<&str> = username.split(USERNAME_SEPARATOR).collect();

        match parts.len() {
            1 => {
                let tenant_id = parts[0].trim().to_string();
                if tenant_id.is_empty() {
                    return Err(AuthError::EmptyTenantId);
                }
                debug!("Extracted tenant_id from username: {}", tenant_id);
                Ok((tenant_id, None))
            }
            2 => {
                let tenant_id = parts[0].trim().to_string();
                let user_id = parts[1].trim().to_string();
                if tenant_id.is_empty() {
                    return Err(AuthError::EmptyTenantId);
                }
                debug!(
                    "Extracted tenant_id '{}' and user_id '{}' from username",
                    tenant_id, user_id
                );
                Ok((tenant_id, Some(user_id)))
            }
            _ => Err(AuthError::InvalidUsernameFormat(format!(
                "Expected format 'tenant_id' or 'tenant_id:user_id', got '{}'",
                username
            ))),
        }
    }

    pub fn extract_from_client_id(
        &self,
        client_id: &str,
    ) -> Result<(String, Option<String>), AuthError> {
        if client_id.is_empty() {
            return Err(AuthError::InvalidClientIdFormat(
                "Client ID is empty".to_string(),
            ));
        }

        // Parse client ID format: tenant_id/device_id or just tenant_id
        let parts: Vec<&str> = client_id.split(CLIENTID_SEPARATOR).collect();

        match parts.len() {
            1 => {
                let tenant_id = parts[0].trim().to_string();
                if tenant_id.is_empty() {
                    return Err(AuthError::EmptyTenantId);
                }
                debug!("Extracted tenant_id from client_id: {}", tenant_id);
                Ok((tenant_id, None))
            }
            2 => {
                let tenant_id = parts[0].trim().to_string();
                let device_id = parts[1].trim().to_string();
                if tenant_id.is_empty() {
                    return Err(AuthError::EmptyTenantId);
                }
                debug!(
                    "Extracted tenant_id '{}' and device_id '{}' from client_id",
                    tenant_id, device_id
                );
                Ok((tenant_id, Some(device_id)))
            }
            _ => {
                // If multiple slashes, take first part as tenant_id
                let tenant_id = parts[0].trim().to_string();
                if tenant_id.is_empty() {
                    return Err(AuthError::EmptyTenantId);
                }
                warn!(
                    "Client ID has unexpected format with multiple separators: {}",
                    client_id
                );
                Ok((tenant_id, None))
            }
        }
    }

    pub fn extract_tenant_context(
        &self,
        client_id: &str,
        username: Option<&str>,
        cert_der: Option<&[u8]>,
        client_ip: Option<IpAddr>,
    ) -> Result<TenantContext, AuthError> {
        let mut tenant_id: Option<String> = None;
        let mut user_id: Option<String> = None;
        let mut device_id: Option<String> = None;
        let mut auth_source: Option<AuthSource> = None;

        // Try certificate extraction if mTLS enabled and cert provided
        if self.enable_mtls {
            if let Some(cert_bytes) = cert_der {
                match self.extract_from_certificate(cert_bytes) {
                    Ok(tid) => {
                        tenant_id = Some(tid);
                        auth_source = Some(AuthSource::Certificate);
                    }
                    Err(e) => {
                        warn!("Failed to extract tenant from certificate: {}", e);
                    }
                }
            }
        }

        // Try username extraction if username provided
        if let Some(uname) = username {
            match self.extract_from_username(uname) {
                Ok((tid, uid)) => {
                    // Verify consistency with certificate if both present
                    if let Some(ref cert_tenant) = tenant_id {
                        if cert_tenant != &tid {
                            return Err(AuthError::TenantIdMismatch {
                                cert_tenant: cert_tenant.clone(),
                                username_tenant: tid,
                            });
                        }
                    } else {
                        tenant_id = Some(tid);
                        auth_source = Some(AuthSource::Username);
                    }
                    user_id = uid;
                }
                Err(e) => {
                    warn!("Failed to extract tenant from username: {}", e);
                }
            }
        }

        // Try client ID extraction as fallback
        if tenant_id.is_none() {
            match self.extract_from_client_id(client_id) {
                Ok((tid, did)) => {
                    tenant_id = Some(tid);
                    device_id = did;
                    auth_source = Some(AuthSource::ClientId);
                }
                Err(e) => {
                    warn!("Failed to extract tenant from client_id: {}", e);
                }
            }
        } else {
            // Extract device_id from client_id if tenant already known
            if let Ok((_, did)) = self.extract_from_client_id(client_id) {
                device_id = did;
            }
        }

        // Return error if no tenant ID was extracted from any source
        let final_tenant_id = tenant_id.ok_or(AuthError::TenantIdNotFound)?;
        let final_auth_source = auth_source.ok_or(AuthError::TenantIdNotFound)?;

        // Build TenantContext with all extracted information
        let mut context = TenantContext::new(final_tenant_id, client_id.to_string(), final_auth_source);

        if let Some(uid) = user_id {
            context = context.with_user_id(uid);
        }
        if let Some(did) = device_id {
            context = context.with_device_id(did);
        }
        if let Some(ip) = client_ip {
            context = context.with_client_ip(ip);
        }

        debug!(
            "Successfully extracted tenant context: tenant_id={}, connection_id={}",
            context.tenant_id, context.connection_id
        );

        Ok(context)
    }
}
