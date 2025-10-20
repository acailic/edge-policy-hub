use crate::ast::{
    AttributeCategory, AttributePath, Condition, Effect, Expression, Operator, Policy,
};

pub fn generate_rego(policy: &Policy, tenant_id: &str) -> String {
    let mut sections = Vec::new();
    sections.push(generate_package_declaration(tenant_id));
    sections.push(generate_import_statement());
    sections.push(generate_default_rule(&policy.effect));
    sections.push(generate_allow_rule(policy, tenant_id));

    sections.join("\n\n")
}

pub fn generate_package_declaration(tenant_id: &str) -> String {
    format!("package tenants.{tenant_id}")
}

pub fn generate_import_statement() -> String {
    "import rego.v1".to_string()
}

pub fn generate_default_rule(effect: &Effect) -> String {
    match effect {
        Effect::Allow => "default allow := false\ndefault deny := true".to_string(),
        Effect::Deny => "default allow := true\ndefault deny := false".to_string(),
    }
}

pub fn generate_allow_rule(policy: &Policy, tenant_id: &str) -> String {
    let rule_name = match policy.effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
    };

    let mut lines = Vec::new();
    lines.push(format!("{rule_name} if {{"));

    for condition in generate_conditions(policy, tenant_id) {
        lines.push(format!("    {condition}"));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

pub fn generate_conditions(policy: &Policy, tenant_id: &str) -> Vec<String> {
    let mut conditions = Vec::new();
    let existing_conditions: Vec<String> =
        policy.conditions.iter().map(generate_condition).collect();

    if !has_tenant_guard(policy, tenant_id) {
        conditions.push(format!(
            "{} == {}",
            generate_attribute_injection_path(),
            generate_expression(&Expression::StringLiteral(tenant_id.to_string()))
        ));
    }

    let action_guard = format!(
        "input.action == {}",
        generate_expression(&Expression::StringLiteral(
            policy.action.as_str().to_string()
        ))
    );
    if !existing_conditions.iter().any(|c| c == &action_guard) {
        conditions.push(action_guard);
    }

    let resource_guard = format!(
        "input.resource.type == {}",
        generate_expression(&Expression::StringLiteral(policy.resource_type.clone()))
    );
    if !existing_conditions.iter().any(|c| c == &resource_guard) {
        conditions.push(resource_guard);
    }

    conditions.extend(existing_conditions);

    conditions
}

pub fn generate_condition(condition: &Condition) -> String {
    format!(
        "{} {} {}",
        generate_expression(&condition.left),
        generate_operator(&condition.operator),
        generate_expression(&condition.right)
    )
}

pub fn generate_operator(operator: &Operator) -> String {
    match operator {
        Operator::Equal => "==",
        Operator::NotEqual => "!=",
        Operator::LessThan => "<",
        Operator::LessThanOrEqual => "<=",
        Operator::GreaterThan => ">",
        Operator::GreaterThanOrEqual => ">=",
        Operator::In => "in",
        Operator::And => "and",
        Operator::Or => "or",
        Operator::Not => "not",
    }
    .to_string()
}

pub fn generate_expression(expression: &Expression) -> String {
    match expression {
        Expression::AttributePath(path) => generate_attribute_path(path),
        Expression::StringLiteral(value) => format!("\"{}\"", escape_string(value)),
        Expression::NumberLiteral(value) => format_number(*value),
        Expression::BooleanLiteral(value) => value.to_string(),
        Expression::ListLiteral(elements) => {
            let rendered: Vec<String> = elements.iter().map(generate_expression).collect();
            format!("[{}]", rendered.join(", "))
        }
    }
}

fn generate_attribute_path(path: &AttributePath) -> String {
    match &path.category {
        AttributeCategory::Subject => format!("input.subject.{}", path.field),
        AttributeCategory::Resource => format!("input.resource.{}", path.field),
        AttributeCategory::Environment => format!("input.environment.{}", path.field),
        AttributeCategory::Action => format!("input.action.{}", path.field),
        AttributeCategory::Custom(category) => {
            format!("input.{}.{}", category, path.field)
        }
    }
}

fn has_tenant_guard(policy: &Policy, tenant_id: &str) -> bool {
    policy.conditions.iter().any(|condition| {
        matches!(
            (&condition.left, &condition.operator, &condition.right),
            (
                Expression::AttributePath(AttributePath {
                    category: AttributeCategory::Subject,
                    field,
                }),
                Operator::Equal,
                Expression::StringLiteral(value),
            ) if field == "tenant_id" && value == tenant_id
        )
    })
}

fn generate_attribute_injection_path() -> String {
    "input.subject.tenant_id".to_string()
}

fn escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value.trunc())
    } else {
        format!("{}", value)
    }
}
