use std::{fs, path::Path};

use edge_policy_enforcer::{
    policy::PolicyManager,
    tenant::{validate_tenant_match, TenantValidationError},
};
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn test_load_tenant_bundle() {
    let temp = tempdir().expect("failed to create temp dir");
    let tenant_dir = temp.path().join("test_tenant");
    fs::create_dir_all(&tenant_dir).expect("failed to create tenant dir");
    write_policy(&tenant_dir, &allow_policy("test_tenant"));

    let manager = PolicyManager::new(temp.path().to_path_buf());
    manager
        .load_tenant("test_tenant")
        .expect("tenant should load");

    let tenants = manager.list_tenants();
    assert!(tenants.contains(&"test_tenant".to_string()));
}

#[tokio::test]
async fn test_evaluate_allow() {
    let temp = tempdir().expect("failed to create temp dir");
    let tenant_dir = temp.path().join("allow_tenant");
    fs::create_dir_all(&tenant_dir).unwrap();
    write_policy(&tenant_dir, &allow_policy("allow_tenant"));

    let manager = PolicyManager::new(temp.path().to_path_buf());
    manager.load_tenant("allow_tenant").unwrap();

    let input = json!({
        "subject": {"tenant_id": "allow_tenant"},
        "action": "read",
    });

    let decision = manager
        .evaluate("allow_tenant", input)
        .await
        .expect("evaluation should succeed");
    assert!(decision.allow);
}

#[tokio::test]
async fn test_evaluate_deny() {
    let temp = tempdir().expect("failed to create temp dir");
    let tenant_dir = temp.path().join("deny_tenant");
    fs::create_dir_all(&tenant_dir).unwrap();
    write_policy(&tenant_dir, &deny_policy("deny_tenant"));

    let manager = PolicyManager::new(temp.path().to_path_buf());
    manager.load_tenant("deny_tenant").unwrap();

    let input = json!({
        "subject": {"tenant_id": "deny_tenant"},
        "action": "read",
    });

    let decision = manager
        .evaluate("deny_tenant", input)
        .await
        .expect("evaluation should succeed");
    assert!(!decision.allow);
}

#[tokio::test]
async fn test_tenant_isolation() {
    let temp = tempdir().expect("failed to create temp dir");
    let allow_dir = temp.path().join("tenant_allow");
    let deny_dir = temp.path().join("tenant_deny");
    fs::create_dir_all(&allow_dir).unwrap();
    fs::create_dir_all(&deny_dir).unwrap();

    write_policy(&allow_dir, &allow_policy("tenant_allow"));
    write_policy(&deny_dir, &deny_policy("tenant_deny"));

    let manager = PolicyManager::new(temp.path().to_path_buf());
    manager.load_tenant("tenant_allow").unwrap();
    manager.load_tenant("tenant_deny").unwrap();

    let allow_input = json!({
        "subject": {"tenant_id": "tenant_allow"},
        "action": "read",
    });
    let allow_decision = manager
        .evaluate("tenant_allow", allow_input)
        .await
        .expect("evaluation should succeed");
    assert!(allow_decision.allow);

    let deny_input = json!({
        "subject": {"tenant_id": "tenant_deny"},
        "action": "read",
    });
    let deny_decision = manager
        .evaluate("tenant_deny", deny_input)
        .await
        .expect("evaluation should succeed");
    assert!(!deny_decision.allow);
}

#[tokio::test]
async fn test_tenant_validation_mismatch() {
    let input = json!({
        "subject": {"tenant_id": "tenant_a"},
    });

    let result = validate_tenant_match("tenant_b", &input);
    assert!(matches!(
        result,
        Err(TenantValidationError::Mismatch { .. })
    ));
}

#[tokio::test]
async fn test_hot_reload() {
    let temp = tempdir().expect("failed to create temp dir");
    let tenant_dir = temp.path().join("reload_tenant");
    fs::create_dir_all(&tenant_dir).unwrap();

    write_policy(&tenant_dir, &deny_policy("reload_tenant"));

    let manager = PolicyManager::new(temp.path().to_path_buf());
    manager.load_tenant("reload_tenant").unwrap();

    let initial_input = json!({
        "subject": {"tenant_id": "reload_tenant"},
        "action": "read",
    });
    let initial = manager
        .evaluate("reload_tenant", initial_input)
        .await
        .expect("evaluation should succeed");
    assert!(!initial.allow);

    write_policy(&tenant_dir, &allow_policy("reload_tenant"));
    manager.reload_tenant("reload_tenant").unwrap();

    let updated_input = json!({
        "subject": {"tenant_id": "reload_tenant"},
        "action": "read",
    });
    let updated = manager
        .evaluate("reload_tenant", updated_input)
        .await
        .expect("evaluation should succeed");
    assert!(updated.allow);
}

fn write_policy(dir: &Path, content: &str) {
    fs::write(dir.join("policy.rego"), content).expect("failed to write policy");
}

fn allow_policy(tenant: &str) -> String {
    format!(
        r#"
package tenants.{tenant}

default allow = false

allow if {{
    input.subject.tenant_id == "{tenant}"
    input.action == "read"
}}
"#,
        tenant = tenant
    )
}

fn deny_policy(tenant: &str) -> String {
    format!(
        r#"
package tenants.{tenant}

default allow = false
"#,
        tenant = tenant
    )
}
