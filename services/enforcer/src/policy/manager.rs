use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;
use tracing::{error, info};

use super::{
    loader::{BundleLoader, PolicyBundle},
    PolicyError, TenantEngine, TenantId,
};
use crate::api::PolicyDecision;

pub struct PolicyManager {
    engines: Arc<RwLock<HashMap<TenantId, TenantEngine>>>,
    bundles_dir: PathBuf,
    loader: BundleLoader,
}

impl PolicyManager {
    pub fn new(bundles_dir: PathBuf) -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            bundles_dir,
            loader: BundleLoader::new(),
        }
    }

    pub fn load_all_tenants(&self) -> Result<usize> {
        let mut count = 0usize;

        for entry in fs::read_dir(&self.bundles_dir).with_context(|| {
            format!(
                "failed to read bundles directory '{}'",
                self.bundles_dir.display()
            )
        })? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let tenant_id = entry.file_name().to_string_lossy().to_string();

            match self.load_tenant(&tenant_id) {
                Ok(_) => {
                    count += 1;
                    info!(tenant = %tenant_id, "loaded tenant policy");
                }
                Err(err) => {
                    error!(tenant = %tenant_id, error = ?err, "failed to load tenant policy");
                }
            }
        }

        Ok(count)
    }

    pub fn load_tenant(&self, tenant_id: &str) -> Result<(), PolicyError> {
        let bundle_path = self.bundles_dir.join(tenant_id);
        let bundle =
            self.loader
                .load_bundle(&bundle_path)
                .map_err(|err| PolicyError::BundleLoadError {
                    tenant_id: tenant_id.to_string(),
                    source: err,
                })?;

        self.install_tenant_engine(tenant_id, bundle)
    }

    pub fn reload_tenant(&self, tenant_id: &str) -> Result<(), PolicyError> {
        let bundle_path = self.bundles_dir.join(tenant_id);
        let bundle =
            self.loader
                .load_bundle(&bundle_path)
                .map_err(|err| PolicyError::BundleLoadError {
                    tenant_id: tenant_id.to_string(),
                    source: err,
                })?;

        self.install_tenant_engine(tenant_id, bundle)
    }

    pub async fn evaluate(
        &self,
        tenant_id: &str,
        input: JsonValue,
    ) -> Result<PolicyDecision, PolicyError> {
        let engine = {
            let guard = self
                .engines
                .read()
                .map_err(|_| PolicyError::EvaluationFailed {
                    tenant_id: tenant_id.to_string(),
                    source: anyhow!("engine map poisoned"),
                })?;
            guard
                .get(tenant_id)
                .cloned()
                .ok_or_else(|| PolicyError::TenantNotFound(tenant_id.to_string()))?
        };

        engine.evaluate(input).await
    }

    pub fn list_tenants(&self) -> Vec<String> {
        self.engines
            .read()
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn install_tenant_engine(
        &self,
        tenant_id: &str,
        bundle: PolicyBundle,
    ) -> Result<(), PolicyError> {
        let engine = TenantEngine::new(tenant_id.to_string(), bundle.policies, bundle.data)?;

        if let Err(err) = engine.verify_entrypoint() {
            error!(
                tenant = %tenant_id,
                error = ?err,
                "tenant entrypoint validation failed"
            );
            return Err(err);
        }

        let mut guard = self
            .engines
            .write()
            .map_err(|_| PolicyError::BundleLoadError {
                tenant_id: tenant_id.to_string(),
                source: anyhow!("engine map poisoned"),
            })?;

        guard.insert(tenant_id.to_string(), engine);

        Ok(())
    }
}
