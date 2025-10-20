use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::debug;
use std::collections::BTreeMap;

use crate::api::types::AuditLogEntry;

use super::error::SigningError;

type HmacSha256 = Hmac<Sha256>;

pub struct Signer {
    key: Vec<u8>,
}

impl Signer {
    pub fn new(secret: &str) -> Result<Self, SigningError> {
        if secret.trim().is_empty() {
            return Err(SigningError::InvalidKey("secret cannot be empty".into()));
        }

        let decoded = base64::decode(secret).unwrap_or_else(|_| secret.as_bytes().to_vec());
        if decoded.len() < 32 {
            return Err(SigningError::InvalidKey(
                "signing key must be at least 32 bytes".into(),
            ));
        }

        Ok(Self { key: decoded })
    }

    pub fn sign(&self, data: &[u8]) -> Result<String, SigningError> {
        let mut mac =
            HmacSha256::new_from_slice(&self.key).map_err(|err| SigningError::InvalidKey(err.to_string()))?;
        mac.update(data);
        let result = mac.finalize().into_bytes();
        Ok(base64::encode(result))
    }

    pub fn verify(&self, data: &[u8], signature: &str) -> Result<bool, SigningError> {
        let decoded = base64::decode(signature)
            .map_err(|err| SigningError::EncodingError(err.to_string()))?;
        let mut mac =
            HmacSha256::new_from_slice(&self.key).map_err(|err| SigningError::InvalidKey(err.to_string()))?;
        mac.update(data);

        Ok(mac
            .verify_slice(&decoded)
            .map(|_| true)
            .unwrap_or(false))
    }

    pub fn sign_audit_log(&self, log: &AuditLogEntry) -> Result<String, SigningError> {
        let payload = canonical_payload(log)?;
        let signature = self.sign(payload.as_bytes())?;
        debug!(
            tenant_id = %log.tenant_id,
            log_id = %log.log_id,
            "generated audit log signature"
        );
        Ok(signature)
    }
}

fn canonical_payload(log: &AuditLogEntry) -> Result<String, SigningError> {
    let subject = canonicalize_json(&log.subject)?;
    let resource = canonicalize_json(&log.resource)?;
    let environment = canonicalize_json(&log.environment)?;

    let policy_version = log
        .policy_version
        .map(|v| v.to_string())
        .unwrap_or_default();
    let reason = log.reason.clone().unwrap_or_default();

    Ok(vec![
        log.log_id.clone(),
        log.tenant_id.clone(),
        log.timestamp.clone(),
        log.decision.clone(),
        log.protocol.clone(),
        subject,
        log.action.clone(),
        resource,
        environment,
        policy_version,
        reason,
    ]
    .join("|"))
}

fn canonicalize_json(value: &serde_json::Value) -> Result<String, SigningError> {
    let sorted = sort_json_keys(value);
    serde_json::to_string(&sorted).map_err(|err| SigningError::EncodingError(err.to_string()))
}

fn sort_json_keys(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), sort_json_keys(v)))
                .collect();
            serde_json::Value::Object(
                sorted.into_iter().collect()
            )
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(
                arr.iter().map(sort_json_keys).collect()
            )
        }
        other => other.clone(),
    }
}
