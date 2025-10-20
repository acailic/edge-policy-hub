use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::debug;

use super::PolicyError;

#[derive(Debug, Clone)]
pub struct BundleLoader;

impl BundleLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load_bundle(&self, bundle_path: &Path) -> Result<PolicyBundle> {
        let metadata = fs::metadata(bundle_path).with_context(|| {
            format!(
                "bundle path '{}' does not exist or is not accessible",
                bundle_path.display()
            )
        })?;

        if !metadata.is_dir() {
            return Err(PolicyError::BundleLoadError {
                tenant_id: bundle_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
                source: anyhow::anyhow!(
                    "bundle path '{}' is not a directory",
                    bundle_path.display()
                ),
            }
            .into());
        }

        let mut policies = Vec::new();
        collect_rego_files(bundle_path, bundle_path, &mut policies)?;

        let data = load_optional_json(bundle_path.join("data.json"))?;
        let metadata = load_optional_json(bundle_path.join("metadata.json"))?;

        let metadata = match metadata {
            Some(json) => Some(serde_json::from_value(json).context("invalid metadata.json")?),
            None => None,
        };

        debug!(
            tenant_bundle = %bundle_path.display(),
            policies = policies.len(),
            has_data = data.is_some(),
            has_metadata = metadata.is_some(),
            "loaded tenant policy bundle"
        );

        Ok(PolicyBundle {
            policies,
            data,
            metadata,
        })
    }
}

fn collect_rego_files(
    directory: &Path,
    root: &Path,
    policies: &mut Vec<(String, String)>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read directory '{}'", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            collect_rego_files(&path, root, policies)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rego") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read policy file '{}'", path.display()))?;

        let relative_path = path.strip_prefix(root).unwrap_or_else(|_| path.as_path());

        policies.push((relative_path.to_string_lossy().to_string(), content));
    }

    Ok(())
}

fn load_optional_json(path: PathBuf) -> Result<Option<JsonValue>> {
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let value = serde_json::from_str(&data)
        .with_context(|| format!("invalid JSON in '{}'", path.display()))?;

    Ok(Some(value))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PolicyBundle {
    pub policies: Vec<(String, String)>,
    pub data: Option<JsonValue>,
    pub metadata: Option<BundleMetadata>,
}
