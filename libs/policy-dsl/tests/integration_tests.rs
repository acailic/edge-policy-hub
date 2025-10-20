//! End-to-end integration tests

use edge_policy_dsl::{compile_policy, BundleBuilder, PolicyDslError, PolicyMetadata};
use tempfile::tempdir;

#[test]
fn test_compile_data_residency_policy() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-eu" and resource.region == "EU" and subject.device_location in ["DE", "FR", "NL"]"#;

    let result = compile_policy(dsl, "tenant-eu", None);
    assert!(result.is_ok());

    let compiled = result.unwrap();
    assert_eq!(compiled.tenant_id, "tenant-eu");
    assert!(compiled.rego.contains("package tenants.tenant-eu"));
    assert!(compiled
        .rego
        .contains("input.subject.tenant_id == \"tenant-eu\""));
    assert!(compiled.rego.contains("input.resource.region == \"EU\""));
    assert!(compiled
        .rego
        .contains("input.subject.device_location in [\"DE\", \"FR\", \"NL\"]"));
}

#[test]
fn test_compile_cost_guardrail_policy() {
    let dsl = r#"deny write sensor_data if environment.bandwidth_used >= 100"#;

    let result = compile_policy(dsl, "tenant-a", None);
    assert!(result.is_ok());

    let compiled = result.unwrap();
    assert!(compiled.rego.contains("deny if {"));
    assert!(compiled
        .rego
        .contains("input.environment.bandwidth_used >= 100"));
}

#[test]
fn test_compile_multi_tenant_separation_policy() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == resource.owner_tenant"#;

    let result = compile_policy(dsl, "tenant-a", None);
    assert!(result.is_ok());

    let compiled = result.unwrap();
    assert!(compiled
        .rego
        .contains("input.subject.tenant_id == input.resource.owner_tenant"));
}

#[test]
fn test_compile_with_metadata() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let metadata = PolicyMetadata {
        version: "1.0.0".to_string(),
        author: Some("admin@example.com".to_string()),
        description: Some("Test policy".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let result = compile_policy(dsl, "tenant-a", Some(metadata.clone()));
    assert!(result.is_ok());

    let compiled = result.unwrap();
    assert_eq!(compiled.metadata.version, "1.0.0");
    assert_eq!(
        compiled.metadata.author,
        Some("admin@example.com".to_string())
    );
    assert_eq!(
        compiled.metadata.description,
        Some("Test policy".to_string())
    );
}

#[test]
fn test_compile_invalid_syntax() {
    let dsl = r#"allow read if subject.tenant_id"#; // Missing resource type
    let result = compile_policy(dsl, "tenant-a", None);
    assert!(result.is_err());
}

#[test]
fn test_compile_invalid_attribute() {
    let dsl = r#"allow read sensor_data if subject.invalid_field == "value""#;
    let result = compile_policy(dsl, "tenant-a", None);
    assert!(result.is_err());
    if let Err(PolicyDslError::InvalidAttribute { .. }) = result {
        // expected
    } else {
        panic!("expected invalid attribute error");
    }
}

#[test]
fn test_bundle_builder_workflow() {
    let dsl1 = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let dsl2 = r#"allow write sensor_data if subject.clearance_level >= 2"#;

    let compiled1 = compile_policy(dsl1, "tenant-a", None).unwrap();
    let compiled2 = compile_policy(dsl2, "tenant-a", None).unwrap();

    let mut builder = BundleBuilder::new("tenant-a".to_string());
    builder
        .add_policy("policy1".to_string(), compiled1.rego)
        .add_policy("policy2".to_string(), compiled2.rego);

    let bundle = builder.build();
    assert!(bundle.is_ok());

    let bundle = bundle.unwrap();
    assert_eq!(bundle.tenant_id, "tenant-a");
    assert_eq!(bundle.policies.len(), 2);
}

#[test]
fn test_bundle_write_to_directory() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let compiled = compile_policy(dsl, "tenant-a", None).unwrap();

    let mut builder = BundleBuilder::new("tenant-a".to_string());
    builder.add_policy("policy".to_string(), compiled.rego);

    let bundle = builder.build().unwrap();

    // Create temporary directory
    let temp_dir = tempdir().unwrap();
    let result = bundle.write_to_directory(temp_dir.path());
    assert!(result.is_ok());

    // Verify directory structure
    let tenant_dir = temp_dir.path().join("tenant-a");
    assert!(tenant_dir.exists());
    assert!(tenant_dir.is_dir());

    // Verify policy file exists
    let policy_file = tenant_dir.join("policy.rego");
    assert!(policy_file.exists());

    // Verify metadata file exists
    let metadata_file = tenant_dir.join("metadata.json");
    assert!(metadata_file.exists());

    // Read and verify policy content
    let policy_content = std::fs::read_to_string(&policy_file).unwrap();
    assert!(policy_content.contains("package tenants.tenant-a"));
}

#[test]
fn test_bundle_with_data() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let compiled = compile_policy(dsl, "tenant-a", None).unwrap();

    let mut builder = BundleBuilder::new("tenant-a".to_string());
    builder
        .add_policy("policy".to_string(), compiled.rego)
        .with_data(serde_json::json!({"roles": ["admin", "operator"]}));

    let bundle = builder.build().unwrap();

    let temp_dir = tempdir().unwrap();
    bundle.write_to_directory(temp_dir.path()).unwrap();

    // Verify data.json exists
    let data_file = temp_dir.path().join("tenant-a").join("data.json");
    assert!(data_file.exists());

    // Read and verify data content
    let data_content = std::fs::read_to_string(&data_file).unwrap();
    assert!(data_content.contains("admin"));
    assert!(data_content.contains("operator"));
}

#[test]
fn test_end_to_end_dsl_to_bundle() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-test" and resource.region == "EU""#;

    // Compile policy
    let compiled = compile_policy(dsl, "tenant-test", None).unwrap();

    // Create bundle
    let mut builder = BundleBuilder::new("tenant-test".to_string());
    builder.add_policy("eu-policy".to_string(), compiled.rego);
    let bundle = builder.build().unwrap();

    // Write to directory
    let temp_dir = tempdir().unwrap();
    bundle.write_to_directory(temp_dir.path()).unwrap();

    // Verify the complete bundle structure
    let tenant_dir = temp_dir.path().join("tenant-test");
    assert!(tenant_dir.exists());
    assert!(tenant_dir.join("eu-policy.rego").exists());
    assert!(tenant_dir.join("metadata.json").exists());

    // Verify content
    let policy_content = std::fs::read_to_string(tenant_dir.join("eu-policy.rego")).unwrap();
    assert!(policy_content.contains("package tenants.tenant-test"));
    assert!(policy_content.contains("input.resource.region == \"EU\""));
}

#[test]
fn test_manifest_json_generation() {
    let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let compiled = compile_policy(dsl, "tenant-a", None).unwrap();

    let mut builder = BundleBuilder::new("tenant-a".to_string());
    builder.add_policy("policy".to_string(), compiled.rego);
    let bundle = builder.build().unwrap();

    let manifest = bundle.to_manifest_json();
    assert!(manifest.is_ok());

    let manifest_str = manifest.unwrap();
    assert!(manifest_str.contains("tenants/tenant-a"));
    assert!(manifest_str.contains("rego_version"));
    assert!(manifest_str.contains("\"revision\""));
}

#[test]
fn test_multiple_policies_same_tenant() {
    let policies = vec![
        r#"allow read sensor_data if subject.clearance_level >= 1"#,
        r#"allow write sensor_data if subject.clearance_level >= 2"#,
        r#"allow delete sensor_data if subject.clearance_level >= 3"#,
    ];

    let mut builder = BundleBuilder::new("tenant-multi".to_string());

    for (i, dsl) in policies.iter().enumerate() {
        let compiled = compile_policy(dsl, "tenant-multi", None).unwrap();
        builder.add_policy(format!("policy{}", i), compiled.rego);
    }

    let bundle = builder.build().unwrap();
    assert_eq!(bundle.policies.len(), 3);

    let temp_dir = tempdir().unwrap();
    bundle.write_to_directory(temp_dir.path()).unwrap();

    // Verify all policy files exist
    let tenant_dir = temp_dir.path().join("tenant-multi");
    assert!(tenant_dir.join("policy0.rego").exists());
    assert!(tenant_dir.join("policy1.rego").exists());
    assert!(tenant_dir.join("policy2.rego").exists());
}
