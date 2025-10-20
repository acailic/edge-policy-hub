//! Edge Policy Hub Policy DSL to Rego compiler.
//!
//! This crate provides a complete DSL-to-Rego compiler for ABAC-style policies.
//! It supports parsing human-readable policy syntax, validating attribute paths,
//! generating Rego code with tenant namespace injection, and packaging bundles
//! for deployment.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Module declarations
pub mod ast;
pub mod bundle;
pub mod codegen;
pub mod parser;
pub mod validator;

// Re-export key types
pub use ast::{
    Action, AttributeCategory, AttributePath, Condition, Effect, Expression, Operator, Policy,
};
pub use bundle::{BundleBuilder, BundleMetadata, PolicyBundle};

/// Policy metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

impl Default for PolicyMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            author: None,
            description: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Describes the compiled representation of a policy artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPolicy {
    /// Human-readable policy identifier.
    pub name: String,
    /// Tenant identifier for namespace injection.
    pub tenant_id: String,
    /// Generated Rego program produced by the compiler.
    pub rego: String,
    /// Policy metadata.
    pub metadata: PolicyMetadata,
}

/// Errors emitted by the policy compiler.
#[derive(Debug, Error)]
pub enum PolicyDslError {
    /// Parse error with location information
    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        location: Option<(usize, usize)>,
    },

    /// Semantic validation error
    #[error("Validation error: {message}")]
    ValidationError {
        message: String,
        attribute: Option<String>,
    },

    /// Invalid attribute path
    #[error("Invalid attribute: {path} - {reason}")]
    InvalidAttribute { path: String, reason: String },

    /// Unsupported operator
    #[error("Unsupported operator: {operator}")]
    UnsupportedOperator { operator: String },

    /// Missing tenant ID
    #[error("Tenant ID is required")]
    TenantIdRequired,

    /// IO error during bundle operations
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

/// Compiles the provided policy source into a [`CompiledPolicy`].
///
/// # Arguments
/// * `source` - DSL policy source code
/// * `tenant_id` - Tenant identifier for namespace injection
/// * `metadata` - Optional policy metadata
///
/// # Returns
/// Compiled policy with generated Rego code or error
///
/// # Example
/// ```
/// use edge_policy_dsl::{compile_policy, PolicyMetadata};
///
/// let dsl = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
/// let result = compile_policy(dsl, "tenant-a", None);
/// assert!(result.is_ok());
/// ```
pub fn compile_policy(
    source: &str,
    tenant_id: &str,
    metadata: Option<PolicyMetadata>,
) -> Result<CompiledPolicy, PolicyDslError> {
    if tenant_id.is_empty() {
        return Err(PolicyDslError::TenantIdRequired);
    }

    // Parse DSL source to AST
    let policy = parser::parse_policy(source)?;

    // Validate AST
    validator::validate_policy(&policy)?;

    // Generate Rego code
    let rego = codegen::generate_rego(&policy, tenant_id);

    // Create compiled policy
    let metadata = metadata.unwrap_or_default();
    let name = format!(
        "{}-{}",
        policy.action.as_str(),
        policy.resource_type.replace(' ', "_")
    );

    Ok(CompiledPolicy {
        name,
        tenant_id: tenant_id.to_string(),
        rego,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_policy() {
        let source = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
        let result = compile_policy(source, "tenant-a", None);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.tenant_id, "tenant-a");
        assert!(compiled.rego.contains("package tenants.tenant-a"));
        assert!(compiled.rego.contains("allow if {"));
    }

    #[test]
    fn test_compile_with_metadata() {
        let source = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
        let metadata = PolicyMetadata {
            version: "2.0.0".to_string(),
            author: Some("test@example.com".to_string()),
            description: Some("Test policy".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = compile_policy(source, "tenant-a", Some(metadata.clone()));
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.metadata.version, "2.0.0");
        assert_eq!(
            compiled.metadata.author,
            Some("test@example.com".to_string())
        );
    }

    #[test]
    fn test_compile_missing_tenant_id() {
        let source = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
        let result = compile_policy(source, "", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PolicyDslError::TenantIdRequired
        ));
    }

    #[test]
    fn test_compile_invalid_syntax() {
        let source = r#"invalid syntax here"#;
        let result = compile_policy(source, "tenant-a", None);
        assert!(result.is_err());
    }
}
