use crate::auth::TenantContext;
use chrono::Utc;
use http::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacInput {
    pub subject: SubjectAttributes,
    pub action: String,
    pub resource: ResourceAttributes,
    pub environment: EnvironmentAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectAttributes {
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub roles: Vec<String>,
    pub clearance_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAttributes {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub owner_tenant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentAttributes {
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_used: Option<f64>,
}

impl AbacInput {
    pub fn from_request(
        ctx: &TenantContext,
        method: &Method,
        path: &str,
        _query: Option<&str>,
    ) -> Self {
        // Build subject from TenantContext
        let subject = SubjectAttributes {
            tenant_id: ctx.tenant_id.clone(),
            user_id: ctx.user_id.clone(),
            device_id: ctx.device_id.clone(),
            roles: ctx.roles.clone(),
            clearance_level: 1, // Default clearance level
        };

        // Map HTTP method to action
        let action = match *method {
            Method::GET | Method::HEAD | Method::OPTIONS => "read",
            Method::POST | Method::PUT | Method::PATCH => "write",
            Method::DELETE => "delete",
            _ => "unknown",
        }
        .to_string();

        // Extract resource type from path
        let resource_type = extract_resource_type(path);
        let resource_id = extract_resource_id(path);

        let resource = ResourceAttributes {
            r#type: resource_type,
            id: resource_id,
            classification: None,
            region: None,
            owner_tenant: ctx.tenant_id.clone(),
        };

        // Build environment attributes
        let environment = EnvironmentAttributes {
            time: Utc::now().to_rfc3339(),
            country: None,
            network: ctx.client_ip.map(|ip| ip.to_string()),
            risk_score: None,
            bandwidth_used: Some(0.0), // Placeholder, will be updated with actual quota data
        };

        Self {
            subject,
            action,
            resource,
            environment,
        }
    }
}

fn extract_resource_type(path: &str) -> String {
    // Extract resource type from path
    // E.g., "/api/sensors/123" -> "sensors"
    //       "/api/data" -> "data"
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.len() >= 2 {
        // Skip "api" or similar prefixes
        if parts[0] == "api" || parts[0] == "v1" || parts[0] == "v2" {
            return parts[1].to_string();
        }
        return parts[0].to_string();
    }

    if !parts.is_empty() && !parts[0].is_empty() {
        return parts[0].to_string();
    }

    "unknown".to_string()
}

fn extract_resource_id(path: &str) -> Option<String> {
    // Extract resource ID from path if present
    // E.g., "/api/sensors/123" -> Some("123")
    //       "/api/data" -> None
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.len() >= 3 {
        // Skip "api" prefix and resource type
        if parts[0] == "api" || parts[0] == "v1" || parts[0] == "v2" {
            return Some(parts[2].to_string());
        }
    }

    if parts.len() >= 2 && parts[0] != "api" && parts[0] != "v1" && parts[0] != "v2" {
        return Some(parts[1].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_type() {
        assert_eq!(extract_resource_type("/api/sensors/123"), "sensors");
        assert_eq!(extract_resource_type("/api/data"), "data");
        assert_eq!(extract_resource_type("/sensors"), "sensors");
        assert_eq!(extract_resource_type("/v1/devices/456"), "devices");
        assert_eq!(extract_resource_type("/"), "unknown");
    }

    #[test]
    fn test_extract_resource_id() {
        assert_eq!(extract_resource_id("/api/sensors/123"), Some("123".to_string()));
        assert_eq!(extract_resource_id("/api/data"), None);
        assert_eq!(extract_resource_id("/sensors/456"), Some("456".to_string()));
        assert_eq!(extract_resource_id("/v1/devices/789"), Some("789".to_string()));
        assert_eq!(extract_resource_id("/"), None);
    }
}
