//! Validator tests for the policy DSL

use edge_policy_dsl::ast::*;
use edge_policy_dsl::validator::validate_policy;

#[test]
fn test_validate_valid_policy() {
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

    let result = validate_policy(&policy);
    assert!(result.is_ok());
}

#[test]
fn test_validate_in_operator_with_list() {
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

    let result = validate_policy(&policy);
    assert!(result.is_ok());
}

#[test]
fn test_validate_in_operator_with_non_list() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "tenant_id".to_string(),
            }),
            operator: Operator::In,
            right: Expression::StringLiteral("tenant-a".to_string()),
        }],
    };

    let result = validate_policy(&policy);
    assert!(result.is_err());
}

#[test]
fn test_validate_all_valid_subject_fields() {
    let fields = vec![
        "tenant_id",
        "user_id",
        "device_id",
        "roles",
        "clearance_level",
        "device_location",
    ];

    for field in fields {
        let policy = Policy {
            effect: Effect::Allow,
            action: Action::Read,
            resource_type: "sensor_data".to_string(),
            conditions: vec![Condition {
                left: Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Subject,
                    field: field.to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("value".to_string()),
            }],
        };

        let result = validate_policy(&policy);
        assert!(result.is_ok(), "Field {} should be valid", field);
    }
}

#[test]
fn test_validate_all_valid_resource_fields() {
    let fields = vec![
        "type",
        "id",
        "classification",
        "region",
        "owner_tenant",
        "owner_user",
    ];

    for field in fields {
        let policy = Policy {
            effect: Effect::Allow,
            action: Action::Read,
            resource_type: "sensor_data".to_string(),
            conditions: vec![Condition {
                left: Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Resource,
                    field: field.to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("value".to_string()),
            }],
        };

        let result = validate_policy(&policy);
        assert!(result.is_ok(), "Field {} should be valid", field);
    }
}

#[test]
fn test_validate_all_valid_environment_fields() {
    let fields = vec![
        "time",
        "geo",
        "network",
        "risk_score",
        "session_trust",
        "country",
        "asn",
        "bandwidth_used",
    ];

    for field in fields {
        let policy = Policy {
            effect: Effect::Allow,
            action: Action::Read,
            resource_type: "sensor_data".to_string(),
            conditions: vec![Condition {
                left: Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Environment,
                    field: field.to_string(),
                }),
                operator: Operator::Equal,
                right: Expression::StringLiteral("value".to_string()),
            }],
        };

        let result = validate_policy(&policy);
        assert!(result.is_ok(), "Field {} should be valid", field);
    }
}

#[test]
fn test_validate_custom_attribute_allowed() {
    // Custom attributes should be allowed (with warning)
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "custom_field".to_string(),
            }),
            operator: Operator::Equal,
            right: Expression::StringLiteral("value".to_string()),
        }],
    };

    let result = validate_policy(&policy);
    // Should succeed but log warning
    assert!(result.is_ok());
}

#[test]
fn test_validate_nested_list_literal() {
    let policy = Policy {
        effect: Effect::Allow,
        action: Action::Read,
        resource_type: "sensor_data".to_string(),
        conditions: vec![Condition {
            left: Expression::AttributePath(AttributePath {
                category: AttributeCategory::Subject,
                field: "roles".to_string(),
            }),
            operator: Operator::In,
            right: Expression::ListLiteral(vec![
                Expression::StringLiteral("admin".to_string()),
                Expression::StringLiteral("operator".to_string()),
            ]),
        }],
    };

    let result = validate_policy(&policy);
    assert!(result.is_ok());
}
