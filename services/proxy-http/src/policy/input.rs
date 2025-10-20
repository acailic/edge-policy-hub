use crate::auth::TenantContext;
use chrono::Utc;
use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use url::form_urlencoded;

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
        query: Option<&str>,
        headers: &HeaderMap,
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

        let mut query_id: Option<String> = None;
        let mut query_region: Option<String> = None;
        let mut query_classification: Option<String> = None;

        if let Some(query_str) = query {
            for (key, value) in form_urlencoded::parse(query_str.as_bytes()) {
                let key_lower = key.to_string().to_ascii_lowercase();
                let value_trimmed = value.trim();
                if value_trimmed.is_empty() {
                    continue;
                }
                let value_owned = value_trimmed.to_string();
                match key_lower.as_str() {
                    "id" => {
                        if query_id.is_none() {
                            query_id = Some(value_owned);
                        }
                    }
                    "region" => {
                        if query_region.is_none() {
                            query_region = Some(value_owned);
                        }
                    }
                    "class" | "classification" => {
                        if query_classification.is_none() {
                            query_classification = Some(value_owned);
                        }
                    }
                    _ => {}
                }
            }
        }

        let resource = ResourceAttributes {
            r#type: resource_type,
            id: resource_id.or(query_id),
            classification: header_value(headers, "x-classification").or(query_classification),
            region: header_value(headers, "x-region").or(query_region),
            owner_tenant: ctx.tenant_id.clone(),
        };

        // Build environment attributes
        let environment = EnvironmentAttributes {
            time: Utc::now().to_rfc3339(),
            country: header_value(headers, "x-geo-country"),
            network: ctx.client_ip.map(|ip| ip.to_string()),
            risk_score: None,
            bandwidth_used: None,
        };

        Self {
            subject,
            action,
            resource,
            environment,
        }
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
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
    use crate::auth::AuthMethod;
    use http::{HeaderMap, HeaderValue};
    use std::net::{IpAddr, Ipv4Addr};

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
        assert_eq!(
            extract_resource_id("/api/sensors/123"),
            Some("123".to_string())
        );
        assert_eq!(extract_resource_id("/api/data"), None);
        assert_eq!(extract_resource_id("/sensors/456"), Some("456".to_string()));
        assert_eq!(
            extract_resource_id("/v1/devices/789"),
            Some("789".to_string())
        );
        assert_eq!(extract_resource_id("/"), None);
    }

    #[test]
    fn test_from_request_uses_headers_for_resource_and_environment() {
        let mut ctx = TenantContext::new("tenant-a".to_string(), AuthMethod::Header);
        ctx.client_ip = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)));

        let mut headers = HeaderMap::new();
        headers.insert("X-Region", HeaderValue::from_static("us-west-2"));
        headers.insert("X-Classification", HeaderValue::from_static("confidential"));
        headers.insert("X-Geo-Country", HeaderValue::from_static("US"));

        let input =
            AbacInput::from_request(&ctx, &Method::GET, "/api/resources/42", None, &headers);

        assert_eq!(input.resource.region.as_deref(), Some("us-west-2"));
        assert_eq!(
            input.resource.classification.as_deref(),
            Some("confidential")
        );
        assert_eq!(input.environment.country.as_deref(), Some("US"));
        assert_eq!(input.environment.network.as_deref(), Some("203.0.113.5"));
    }

    #[test]
    fn test_from_request_falls_back_to_query_parameters() {
        let ctx = TenantContext::new("tenant-b".to_string(), AuthMethod::Header);
        let headers = HeaderMap::new();

        let input = AbacInput::from_request(
            &ctx,
            &Method::GET,
            "/api/resources",
            Some("id=abc123&region=eu-central-1&class=restricted"),
            &headers,
        );

        assert_eq!(input.resource.id.as_deref(), Some("abc123"));
        assert_eq!(input.resource.region.as_deref(), Some("eu-central-1"));
        assert_eq!(input.resource.classification.as_deref(), Some("restricted"));
    }
}
