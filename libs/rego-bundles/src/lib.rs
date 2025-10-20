//! Embedded Rego policy bundles for Edge Policy Hub.
//!
//! This library provides reusable Rego policy modules and templates for common patterns:
//! - **Helper Modules** (`lib/`): Reusable functions for geo validation, quota checks, tenant isolation, and time-based access control
//! - **Template Policies** (`templates/`): Complete policy examples demonstrating data residency, cost guardrails, and multi-tenant separation
//! - **OPA Tests** (`tests/`): Comprehensive unit tests for all modules and templates
//!
//! All policies are embedded at compile-time using `include_dir` and exposed through library API functions.
//!
//! ## Structure
//!
//! - `lib/` - Reusable helper modules (geo, quota, tenant, time)
//! - `templates/` - Complete policy templates for reference and adaptation
//! - `tests/` - OPA unit tests
//!
//! ## Usage
//!
//! ```rust
//! use edge_policy_rego_bundles::{list_helpers, load_helper, load_template_policy};
//!
//! // List available helpers
//! let helpers = list_helpers();
//!
//! // Load a specific helper
//! let geo_module = load_helper("geo").expect("geo helper not found");
//!
//! // Load a template policy
//! let template = load_template_policy("data_residency").expect("template not found");
//! ```

use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use tracing::warn;

static POLICIES: Dir = include_dir!("$CARGO_MANIFEST_DIR/policies");

/// Policy category for filtering embedded policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCategory {
    /// Reusable helper modules (lib/)
    Helper,
    /// Complete policy templates (templates/)
    Template,
    /// Test files (tests/)
    Test,
}

/// Returns the list of embedded policy template filenames (deprecated, use list_template_policies).
///
/// # Deprecation Notice
///
/// This function is deprecated and will be removed in a future release.
/// Use [`list_template_policies`] instead, which correctly filters only files
/// from the `templates/` directory.
///
/// This legacy function returns filenames from all subdirectories, which can
/// lead to unexpected results with the new directory structure (`lib/`, `templates/`, `tests/`).
///
/// # Migration
///
/// Replace:
/// ```ignore
/// let templates = available_templates();
/// let content = load_template(name);
/// ```
///
/// With:
/// ```ignore
/// let templates = list_template_policies();
/// let content = load_template_policy(name);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use list_template_policies() instead. This function returns files from all directories, not just templates/."
)]
pub fn available_templates() -> Vec<&'static str> {
    warn!(
        target: "edge-policy-rego-bundles",
        "available_templates() is deprecated; use list_template_policies() for template-only filtering"
    );
    POLICIES
        .files()
        .filter_map(|file| file.path().file_name()?.to_str())
        .collect()
}

/// Loads the raw Rego template content for the given filename (deprecated, use load_template_policy).
///
/// # Deprecation Notice
///
/// This function is deprecated and will be removed in a future release.
/// Use [`load_template_policy`] instead, which correctly loads files
/// from the `templates/` directory using logical names.
///
/// This legacy function expects a full filename and searches across all directories,
/// which can lead to unexpected results with the new directory structure.
///
/// # Migration
///
/// Replace:
/// ```ignore
/// let content = load_template("data_residency.rego");
/// ```
///
/// With:
/// ```ignore
/// let content = load_template_policy("data_residency");
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use load_template_policy() instead. This function searches all directories, not just templates/."
)]
pub fn load_template(name: &str) -> Option<&'static str> {
    warn!(
        target: "edge-policy-rego-bundles",
        "load_template() is deprecated; use load_template_policy() for template-specific loading"
    );
    let file = POLICIES.get_file(name)?;
    let contents = file.contents_utf8();
    if contents.is_none() {
        warn!(target: "edge-policy-rego-bundles", "policy template {name} is not valid UTF-8");
    }
    contents
}

/// Returns the list of helper module names from lib/ subdirectory.
///
/// Example: ["geo", "quota", "tenant", "time"]
pub fn list_helpers() -> Vec<&'static str> {
    POLICIES
        .files()
        .filter_map(|file| {
            let path = file.path();
            // Check if file is in lib/ subdirectory
            if path.starts_with("lib/") && path.extension()? == "rego" {
                // Extract filename without extension
                path.file_stem()?.to_str()
            } else {
                None
            }
        })
        .collect()
}

/// Loads a helper module by name from lib/ subdirectory.
///
/// Example: `load_helper("geo")` loads `lib/geo.rego`
pub fn load_helper(name: &str) -> Option<&'static str> {
    let path = format!("lib/{}.rego", name);
    let file = POLICIES.get_file(&path)?;
    let contents = file.contents_utf8();
    if contents.is_none() {
        warn!(target: "edge-policy-rego-bundles", "helper module {name} is not valid UTF-8");
    }
    contents
}

/// Returns the list of template policy names from templates/ subdirectory.
///
/// Example: ["data_residency", "cost_guardrail", "multi_tenant_separation", "combined_guardrails"]
pub fn list_template_policies() -> Vec<&'static str> {
    POLICIES
        .files()
        .filter_map(|file| {
            let path = file.path();
            // Check if file is in templates/ subdirectory
            if path.starts_with("templates/") && path.extension()? == "rego" {
                // Extract filename without extension
                path.file_stem()?.to_str()
            } else {
                None
            }
        })
        .collect()
}

/// Loads a template policy by name from templates/ subdirectory.
///
/// Example: `load_template_policy("data_residency")` loads `templates/data_residency.rego`
pub fn load_template_policy(name: &str) -> Option<&'static str> {
    let path = format!("templates/{}.rego", name);
    let file = POLICIES.get_file(&path)?;
    let contents = file.contents_utf8();
    if contents.is_none() {
        warn!(target: "edge-policy-rego-bundles", "template policy {name} is not valid UTF-8");
    }
    contents
}

/// Loads all helper modules into a HashMap.
///
/// Returns a HashMap with helper names as keys and Rego source as values.
/// Useful for bulk loading helpers for deployment in tenant bundles.
pub fn load_all_helpers() -> HashMap<String, &'static str> {
    list_helpers()
        .into_iter()
        .filter_map(|name| {
            let contents = load_helper(name)?;
            Some((name.to_string(), contents))
        })
        .collect()
}

/// Returns files filtered by category.
///
/// Filters embedded policies by category (Helper, Template, or Test).
pub fn list_by_category(category: PolicyCategory) -> Vec<&'static str> {
    let prefix = match category {
        PolicyCategory::Helper => "lib/",
        PolicyCategory::Template => "templates/",
        PolicyCategory::Test => "tests/",
    };

    POLICIES
        .files()
        .filter_map(|file| {
            let path = file.path();
            if path.starts_with(prefix) && path.extension()? == "rego" {
                path.file_stem()?.to_str()
            } else {
                None
            }
        })
        .collect()
}
