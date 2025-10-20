use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::auth::TenantContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttAbacInput {
    pub subject: SubjectAttributes,
    pub action: String,
    pub resource: MqttResourceAttributes,
    pub environment: MqttEnvironmentAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectAttributes {
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub clearance_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttResourceAttributes {
    pub r#type: String,
    pub topic: String,
    pub qos: u8,
    pub retain: bool,
    pub owner_tenant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttEnvironmentAttributes {
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_size: Option<usize>,
}

impl MqttAbacInput {
    pub fn for_publish(
        ctx: &TenantContext,
        topic: &str,
        qos: u8,
        retain: bool,
        payload_size: usize,
        message_count: u64,
    ) -> Self {
        // Extract owner_tenant from topic (first segment before /)
        let owner_tenant = topic
            .split('/')
            .next()
            .unwrap_or(&ctx.tenant_id)
            .to_string();

        Self {
            subject: SubjectAttributes {
                tenant_id: ctx.tenant_id.clone(),
                user_id: ctx.user_id.clone(),
                device_id: ctx.device_id.clone(),
                roles: vec![],
                clearance_level: Some(1),
            },
            action: "publish".to_string(),
            resource: MqttResourceAttributes {
                r#type: "mqtt_topic".to_string(),
                topic: topic.to_string(),
                qos,
                retain,
                owner_tenant,
            },
            environment: MqttEnvironmentAttributes {
                time: Utc::now().to_rfc3339(),
                network: ctx.client_ip.map(|ip| ip.to_string()),
                message_count: Some(message_count),
                payload_size: Some(payload_size),
            },
        }
    }

    pub fn for_subscribe(
        ctx: &TenantContext,
        topic_filter: &str,
        qos: u8,
        message_count: u64,
    ) -> Self {
        // Extract owner_tenant from topic filter (first segment before /)
        let owner_tenant = topic_filter
            .split('/')
            .next()
            .filter(|s| *s != "+" && *s != "#")
            .unwrap_or(&ctx.tenant_id)
            .to_string();

        Self {
            subject: SubjectAttributes {
                tenant_id: ctx.tenant_id.clone(),
                user_id: ctx.user_id.clone(),
                device_id: ctx.device_id.clone(),
                roles: vec![],
                clearance_level: Some(1),
            },
            action: "subscribe".to_string(),
            resource: MqttResourceAttributes {
                r#type: "mqtt_topic".to_string(),
                topic: topic_filter.to_string(),
                qos,
                retain: false,
                owner_tenant,
            },
            environment: MqttEnvironmentAttributes {
                time: Utc::now().to_rfc3339(),
                network: ctx.client_ip.map(|ip| ip.to_string()),
                message_count: Some(message_count),
                payload_size: None,
            },
        }
    }
}
