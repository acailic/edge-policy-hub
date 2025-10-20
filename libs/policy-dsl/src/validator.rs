use crate::{
    ast::{AttributeCategory, AttributePath, Condition, Expression, Operator, Policy},
    PolicyDslError,
};
/// Valid attribute fields by category as defined in the README.
const SUBJECT_FIELDS: &[&str] = &[
    "tenant_id",
    "user_id",
    "device_id",
    "roles",
    "clearance_level",
    "device_location",
    "department",
    "region",
];

const RESOURCE_FIELDS: &[&str] = &[
    "type",
    "id",
    "classification",
    "region",
    "owner_tenant",
    "owner_user",
    "sensitivity",
];

const ACTION_FIELDS: &[&str] = &["name", "method", "operation"];

const ENVIRONMENT_FIELDS: &[&str] = &[
    "time",
    "geo",
    "network",
    "risk_score",
    "session_trust",
    "country",
    "asn",
    "bandwidth_used",
];

pub fn validate_policy(policy: &Policy) -> Result<(), PolicyDslError> {
    if policy.conditions.is_empty() {
        return Err(PolicyDslError::ValidationError {
            message: "policy must contain at least one condition".into(),
            attribute: None,
        });
    }

    validate_conditions(&policy.conditions)?;
    Ok(())
}

pub fn validate_conditions(conditions: &[Condition]) -> Result<(), PolicyDslError> {
    for condition in conditions {
        validate_condition(condition)?;
    }
    Ok(())
}

pub fn validate_condition(condition: &Condition) -> Result<(), PolicyDslError> {
    if !matches!(condition.left, Expression::AttributePath(_)) {
        return Err(PolicyDslError::ValidationError {
            message: "left-hand side of a condition must be an attribute path".into(),
            attribute: None,
        });
    }

    validate_expression(&condition.left)?;
    validate_expression(&condition.right)?;
    check_operator_compatibility(&condition.operator, &condition.right)?;
    Ok(())
}

pub fn validate_expression(expression: &Expression) -> Result<(), PolicyDslError> {
    match expression {
        Expression::AttributePath(path) => validate_attribute_path(path),
        Expression::StringLiteral(_) => Ok(()),
        Expression::NumberLiteral(_) => Ok(()),
        Expression::BooleanLiteral(_) => Ok(()),
        Expression::ListLiteral(elements) => {
            for element in elements {
                validate_expression(element)?;
            }
            Ok(())
        }
    }
}

pub fn validate_attribute_path(path: &AttributePath) -> Result<(), PolicyDslError> {
    if path.field.is_empty() {
        return Err(PolicyDslError::InvalidAttribute {
            path: format!("{}.{}", path.category.as_str(), path.field),
            reason: "attribute field cannot be empty".into(),
        });
    }

    let allowed = match &path.category {
        AttributeCategory::Subject => SUBJECT_FIELDS,
        AttributeCategory::Resource => RESOURCE_FIELDS,
        AttributeCategory::Environment => ENVIRONMENT_FIELDS,
        AttributeCategory::Action => ACTION_FIELDS,
        AttributeCategory::Custom(_) => &[][..],
    };

    if !allowed.is_empty() && !allowed.contains(&path.field.as_str()) {
        if path.field.starts_with("custom_") {
            tracing::warn!(
                category = %path.category.as_str(),
                field = %path.field,
                "Unknown attribute treated as custom per naming convention."
            );
        } else {
            return Err(PolicyDslError::InvalidAttribute {
                path: format!("{}.{}", path.category.as_str(), path.field),
                reason: "attribute not listed in approved schema; use `custom_` prefix for custom fields"
                    .into(),
            });
        }
    }

    Ok(())
}

pub fn check_operator_compatibility(
    operator: &Operator,
    right: &Expression,
) -> Result<(), PolicyDslError> {
    match operator {
        Operator::In => match right {
            Expression::ListLiteral(_) => Ok(()),
            _ => Err(PolicyDslError::ValidationError {
                message: "operator `in` requires a list literal on the right-hand side".into(),
                attribute: None,
            }),
        },
        Operator::And | Operator::Or | Operator::Not => Err(PolicyDslError::ValidationError {
            message: format!("logical operator `{operator}` cannot be used as a comparison"),
            attribute: None,
        }),
        _ => Ok(()),
    }
}
