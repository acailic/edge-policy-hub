//! Parser tests for the policy DSL

use edge_policy_dsl::ast::*;
use edge_policy_dsl::parser::parse_policy;

#[test]
fn test_parse_simple_allow_policy() {
    let input = r#"allow read sensor_data if subject.tenant_id == "tenant-a""#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();

    assert!(matches!(policy.effect, Effect::Allow));
    assert!(matches!(policy.action, Action::Read));
    assert_eq!(policy.resource_type, "sensor_data");
    assert_eq!(policy.conditions.len(), 1);

    let condition = &policy.conditions[0];
    assert!(matches!(condition.operator, Operator::Equal));
}

#[test]
fn test_parse_multiple_conditions() {
    let input =
        r#"allow read sensor_data if subject.tenant_id == "tenant-a" and resource.region == "EU""#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();
    assert_eq!(policy.conditions.len(), 2);
}

#[test]
fn test_parse_in_operator() {
    let input = r#"allow read sensor_data if subject.device_location in ["DE", "FR", "NL"]"#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();

    let condition = &policy.conditions[0];
    assert!(matches!(condition.operator, Operator::In));

    if let Expression::ListLiteral(elements) = &condition.right {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("Expected ListLiteral");
    }
}

#[test]
fn test_parse_numeric_comparison() {
    let input = r#"allow read sensor_data if subject.clearance_level >= 2"#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();

    let condition = &policy.conditions[0];
    assert!(matches!(condition.operator, Operator::GreaterThanOrEqual));

    if let Expression::NumberLiteral(n) = condition.right {
        assert_eq!(n, 2.0);
    } else {
        panic!("Expected NumberLiteral");
    }
}

#[test]
fn test_parse_deny_policy() {
    let input = r#"deny write sensor_data if environment.risk_score > 0.8"#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();

    assert!(matches!(policy.effect, Effect::Deny));
    assert!(matches!(policy.action, Action::Write));
}

#[test]
fn test_parse_error_invalid_syntax() {
    let input = r#"allow read if subject.tenant_id"#; // missing resource type
    let result = parse_policy(input);

    assert!(result.is_err());
}

#[test]
fn test_parse_complex_expression() {
    let input = r#"allow read sensor_data if subject.tenant_id == "tenant-a" and resource.region == "EU" or subject.roles in ["admin"]"#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();
    assert!(policy.conditions.len() >= 2);
}

#[test]
fn test_parse_all_actions() {
    let actions = vec![
        ("read", Action::Read),
        ("write", Action::Write),
        ("delete", Action::Delete),
        ("execute", Action::Execute),
        ("subscribe", Action::Subscribe),
        ("publish", Action::Publish),
    ];

    for (action_str, expected_action) in actions {
        let input = format!(
            r#"allow {} sensor_data if subject.tenant_id == "test""#,
            action_str
        );
        let result = parse_policy(&input);
        assert!(result.is_ok());
        let policy = result.unwrap();
        assert_eq!(
            format!("{:?}", policy.action),
            format!("{:?}", expected_action)
        );
    }
}

#[test]
fn test_parse_boolean_literal() {
    let input = r#"allow read sensor_data if subject.active == true"#;
    let result = parse_policy(input);

    assert!(result.is_ok());
    let policy = result.unwrap();

    let condition = &policy.conditions[0];
    if let Expression::BooleanLiteral(b) = condition.right {
        assert_eq!(b, true);
    } else {
        panic!("Expected BooleanLiteral");
    }
}

#[test]
fn test_parse_whitespace_handling() {
    let input = r#"
        allow   read   sensor_data   if
        subject.tenant_id   ==   "tenant-a"
    "#;
    let result = parse_policy(input);

    assert!(result.is_ok());
}
