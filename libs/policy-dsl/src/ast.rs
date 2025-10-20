use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a single policy declaration in the DSL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Policy {
    pub effect: Effect,
    pub action: Action,
    pub resource_type: String,
    pub conditions: Vec<Condition>,
}

/// Describes an individual condition that must be satisfied for the policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    pub left: Expression,
    pub operator: Operator,
    pub right: Expression,
}

/// The effect of the policy (allow or deny).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Effect {
    Allow,
    Deny,
}

/// Supported actions within the DSL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
    Subscribe,
    Publish,
    Custom(String),
}

impl Action {
    pub fn as_str(&self) -> &str {
        match self {
            Action::Read => "read",
            Action::Write => "write",
            Action::Delete => "delete",
            Action::Execute => "execute",
            Action::Subscribe => "subscribe",
            Action::Publish => "publish",
            Action::Custom(value) => value.as_str(),
        }
    }
}

/// Operators supported by the DSL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    In,
    And,
    Or,
    Not,
}

impl Operator {
    pub fn as_str(&self) -> &str {
        match self {
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
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Expressions used in conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Expression {
    AttributePath(AttributePath),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ListLiteral(Vec<Expression>),
}

/// An attribute path such as `subject.tenant_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributePath {
    pub category: AttributeCategory,
    pub field: String,
}

/// High level attribute categories supported by the DSL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttributeCategory {
    Subject,
    Resource,
    Environment,
    Action,
    Custom(String),
}

impl AttributeCategory {
    pub fn as_str(&self) -> &str {
        match self {
            AttributeCategory::Subject => "subject",
            AttributeCategory::Resource => "resource",
            AttributeCategory::Environment => "environment",
            AttributeCategory::Action => "action",
            AttributeCategory::Custom(value) => value.as_str(),
        }
    }
}
