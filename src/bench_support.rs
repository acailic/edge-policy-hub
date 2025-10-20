use edge_policy_enforcer::PolicyManager;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// Re-export external crates needed by harness module
pub use edge_policy_dsl;
pub use md5;
pub use serde_json;
pub use tracing_subscriber;
pub use uuid;

// Re-export the e2e harness module
#[path = "../tests/e2e/harness.rs"]
pub mod e2e_harness;

pub use e2e_harness::{
    default_tenant_config, random_tenant_id, HarnessTempDirs, PortConfig, ServiceConfig,
    ServiceProcess, TestHarness,
};

pub struct PolicyBenchFixture {
    pub manager: Arc<PolicyManager>,
    pub tenant_id: String,
    pub temp_dir: TempDir,
}

impl PolicyBenchFixture {
    pub fn new(tenant_id: &str, rego_source: &str) -> Self {
        let temp_dir = TempDir::new().expect("tempdir");
        let bundles_dir = temp_dir.path().to_path_buf();
        create_bundle(&bundles_dir, tenant_id, rego_source);
        let manager = PolicyManager::new(bundles_dir.clone());
        manager
            .reload_tenant(tenant_id)
            .expect("tenant reloaded for fixture");
        Self {
            manager: Arc::new(manager),
            tenant_id: tenant_id.to_string(),
            temp_dir,
        }
    }
}

fn create_bundle(bundles_dir: &PathBuf, tenant_id: &str, rego: &str) {
    let tenant_dir = bundles_dir.join(tenant_id);
    std::fs::create_dir_all(&tenant_dir).expect("create tenant bundle dir");

    let metadata = json!({
        "version": "bench",
        "author": "bench-suite",
        "description": format!("Benchmark policy bundle for {tenant_id}"),
        "created_at": "1970-01-01T00:00:00Z"
    });
    std::fs::write(tenant_dir.join("metadata.json"), metadata.to_string())
        .expect("write metadata");
    std::fs::write(tenant_dir.join("policy.rego"), rego).expect("write rego");
    std::fs::write(
        tenant_dir.join("data.json"),
        json!({
            "quota": {
                "message_limit": 100_000,
                "bandwidth_limit": 1_000
            }
        })
        .to_string(),
    )
    .expect("write data");
}
