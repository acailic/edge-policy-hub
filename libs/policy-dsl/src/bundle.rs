use crate::PolicyDslError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub revision: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub roots: Vec<String>,
}

impl BundleMetadata {
    pub fn for_tenant(tenant_id: &str) -> Self {
        let now: DateTime<Utc> = Utc::now();
        Self {
            revision: now.timestamp_millis().to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
            created_at: now.to_rfc3339(),
            roots: vec![format!("tenants/{tenant_id}")],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundle {
    pub tenant_id: String,
    pub policies: Vec<(String, String)>,
    pub data: Option<Value>,
    pub metadata: BundleMetadata,
}

impl PolicyBundle {
    pub fn write_to_directory(&self, base_path: &Path) -> Result<(), PolicyDslError> {
        let tenant_dir = base_path.join(&self.tenant_id);
        fs::create_dir_all(&tenant_dir).map_err(|source| PolicyDslError::IoError { source })?;

        for (name, rego) in &self.policies {
            let policy_path = tenant_dir.join(format!("{name}.rego"));
            fs::write(&policy_path, rego).map_err(|source| PolicyDslError::IoError { source })?;
        }

        let metadata_path = tenant_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&self.metadata).map_err(|err| {
            PolicyDslError::IoError {
                source: std::io::Error::new(std::io::ErrorKind::Other, err.to_string()),
            }
        })?;
        fs::write(&metadata_path, metadata_json)
            .map_err(|source| PolicyDslError::IoError { source })?;

        if let Some(data) = &self.data {
            let data_path = tenant_dir.join("data.json");
            let payload =
                serde_json::to_string_pretty(data).map_err(|err| PolicyDslError::IoError {
                    source: std::io::Error::new(std::io::ErrorKind::Other, err.to_string()),
                })?;
            fs::write(data_path, payload).map_err(|source| PolicyDslError::IoError { source })?;
        }

        let manifest_path = tenant_dir.join(".manifest");
        let manifest = self.to_manifest_json()?;
        fs::write(manifest_path, manifest).map_err(|source| PolicyDslError::IoError { source })?;

        Ok(())
    }

    pub fn to_manifest_json(&self) -> Result<String, PolicyDslError> {
        let policy_paths: Vec<String> = self
            .policies
            .iter()
            .map(|(name, _)| format!("tenants/{}/{}.rego", self.tenant_id, name))
            .collect();

        let manifest = json!({
            "revision": self.metadata.revision,
            "roots": self.metadata.roots,
            "rego_version": "v1",
            "metadata": {
                "tenant_id": self.tenant_id,
                "version": self.metadata.version,
                "author": self.metadata.author,
                "description": self.metadata.description,
                "created_at": self.metadata.created_at,
            },
            "policies": policy_paths,
        });

        serde_json::to_string_pretty(&manifest).map_err(|err| PolicyDslError::IoError {
            source: std::io::Error::new(std::io::ErrorKind::Other, err.to_string()),
        })
    }
}

#[derive(Debug)]
pub struct BundleBuilder {
    tenant_id: String,
    policies: Vec<(String, String)>,
    data: Option<Value>,
    metadata: Option<BundleMetadata>,
}

impl BundleBuilder {
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            policies: Vec::new(),
            data: None,
            metadata: None,
        }
    }

    pub fn add_policy(&mut self, name: String, rego: String) -> &mut Self {
        self.policies.push((name, rego));
        self
    }

    pub fn with_data(&mut self, data: Value) -> &mut Self {
        self.data = Some(data);
        self
    }

    pub fn with_metadata(&mut self, metadata: BundleMetadata) -> &mut Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Result<PolicyBundle, PolicyDslError> {
        if self.policies.is_empty() {
            return Err(PolicyDslError::ValidationError {
                message: "bundle must contain at least one policy".into(),
                attribute: None,
            });
        }

        let metadata = self
            .metadata
            .unwrap_or_else(|| BundleMetadata::for_tenant(&self.tenant_id));

        Ok(PolicyBundle {
            tenant_id: self.tenant_id,
            policies: self.policies,
            data: self.data,
            metadata,
        })
    }
}
