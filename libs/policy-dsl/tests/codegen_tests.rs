//! Code generation tests for the policy DSL

use edge_policy_dsl::ast::*;
use edge_policy_dsl::codegen::generate_rego;

#[test]
fn test_generate_simple_policy() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "tenant_id".to_string(),
            }),
            operator: Operator::Equal,
            right: Expression::StringLiteral("tenant-a".to_string()),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("package tenants.tenant-a"));
    assert!(rego.contains("import rego.v1"));
    assert!(rego.contains("default allow := false"));
    assert!(rego.contains("allow if {"));
    assert!(rego.contains("input.subject.tenant_id == \"tenant-a\""));
}

#[test]
fn test_generate_multiple_conditions() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![
            Condition {
                left: Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Subject,
                    field: "tenant_id".to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("tenant-a".to_string()),
            },
            Condition {
                left: Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Resource,
                    field: "region".to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("EU".to_string()),
            },
        ],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("input.subject.tenant_id == \"tenant-a\""));
    assert!(rego.contains("input.resource.region == \"EU\""));
}

#[test]
fn test_generate_in_operator() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "device_location".to_string(),
            }),
            operator: Operator::In,
            right: Expression::ListLiteral(vec![
                Expression::StringLiteral("DE".to_string()),
                Expression::StringLiteral("FR".to_string()),
            ]),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("input.subject.device_location in [\"DE\", \"FR\"]"));
}

#[test]
fn test_generate_numeric_comparison() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "clearance_level".to_string(),
            }),
            operator: Operator::GreaterThanOrEqual,
            right: Expression::NumberLiteral(2.0),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("input.subject.clearance_level >= 2"));
}

#[test]
fn test_generate_tenant_namespace() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "tenant_id".to_string(),
            }),
            operator: Operator::Equal,
            right: Expression::StringLiteral("test".to_string()),
        }],
    };

    let tenants = vec!["tenant-a", "tenant-b", "tenant_123"];

    for tenant_id in tenants {
        let rego = generate_rego(&policy, tenant_id);
        assert!(rego.contains(&format!("package tenants.{}", tenant_id)));
    }
}

#[test]
fn test_generate_deny_policy() {
    let policy = Policy {
        effect: Effect::Deny,
        action: Action::Write,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Environment,
                field: "risk_score".to_string(),
            }),
            operator: Operator::GreaterThan,
            right: Expression::NumberLiteral(0.8),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("default deny := false"));
    assert!(rego.contains("deny if {"));
}

#[test]
fn test_generate_escaped_strings() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "name".to_string(),
            }),
            operator: Operator::Equal,
            right: Expression::StringLiteral("test\"quote".to_string()),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("\\\""));
}

#[test]
fn test_generate_attribute_paths() {
    let categories = vec![
        (
            AttributeCategory::Subject,
            "tenant_id",
            "input.subject.tenant_id",
        ),
        (
            AttributeCategory::Resource,
            "region",
            "input.resource.region",
        ),
        (
            AttributeCategory::Environment,
            "time",
            "input.environment.time",
        ),
    ];

    for (category, field, expected) in categories {
        let policy = Policy {
            effect: Effect::Allow,
            action: Action::Read,
            resource_type: "sensor_data".to_string(),
            conditions: vec![Condition {
                left: Expression::AttributePath(AttributePath {
                    category,
                    field: field.to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("value".to_string()),
            }],
        };

        let rego = generate_rego(&policy, "tenant-a");
        assert!(
            rego.contains(expected),
            "Expected {} in generated Rego",
            expected
        );
    }
}

#[test]
fn test_generate_boolean_literal() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "active".to_string(),
            }),
            operator: Operator::Equal,
            right: Expression::BooleanLiteral(true),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("input.subject.active == true"));
}

#[test]
fn test_generate_decimal_number() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Environment,
                field: "risk_score".to_string(),
            }),
            operator: Operator::LessThan,
            right: Expression::NumberLiteral(0.5),
        }],
    };

    let rego = generate_rego(&policy, "tenant-a");

    assert!(rego.contains("0.5"));
}
