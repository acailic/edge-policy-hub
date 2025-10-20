use crate::{
    ast::{
        Action, AttributeCategory, AttributePath, Condition, Effect, Expression, Operator, Policy,
    },
    PolicyDslError,
};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, tag_no_case, take_while, take_while1},
    character::complete::{char, digit1, multispace0, one_of},
    combinator::{cut, map, map_res, opt, recognize},
    error::{convert_error, VerboseError},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, separated_pair, tuple},
    IResult,
};
use std::num::ParseFloatError;

type Res<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

pub fn parse_policy(source: &str) -> Result<Policy, PolicyDslError> {
    let cleaned = strip_comments(source);
    let input = cleaned.trim();
    if input.is_empty() {
        return Err(PolicyDslError::ParseError {
            message: "policy source is empty".into(),
            location: None,
        });
    }

    match policy_parser(input) {
        Ok((remaining, mut policy)) => {
            let remaining = remaining.trim();
            if !remaining.is_empty() {
                let offset = input.len() - remaining.len();
                return Err(PolicyDslError::ParseError {
                    message: format!("unexpected trailing input: {remaining:?}"),
                    location: compute_location(input, offset),
                });
            }

            // Ensure conditions vector exists even when missing `if`.
            if policy.conditions.is_empty() {
                policy.conditions = Vec::new();
            }

            Ok(policy)
        }
        Err(err) => {
            let (message, location) = match err {
                nom::Err::Error(e) | nom::Err::Failure(e) => {
                    let message = convert_error(input, e.clone());
                    let location = e.errors.first().and_then(|(fragment, _)| {
                        let offset = input.len().saturating_sub(fragment.len());
                        compute_location(input, offset)
                    });
                    (message, location)
                }
                nom::Err::Incomplete(_) => ("incomplete input".to_string(), None),
            };
            Err(PolicyDslError::ParseError { message, location })
        }
    }
}

fn strip_comments(source: &str) -> String {
    let mut result = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            result.push(String::new());
        } else {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}

pub fn policy_parser(input: &str) -> Res<'_, Policy> {
    let (input, effect) = ws(effect_parser)(input)?;
    let (input, action) = ws(action_parser)(input)?;
    let (input, resource_type) = ws(identifier)(input)?;
    let (input, conditions) = opt(preceded(ws(tag_no_case("if")), conditions_parser))(input)?;

    Ok((
        input,
        Policy {
            effect,
            action,
            resource_type: resource_type.to_string(),
            conditions: conditions.unwrap_or_default(),
        },
    ))
}

fn effect_parser(input: &str) -> Res<'_, Effect> {
    alt((
        map(tag_no_case("allow"), |_| Effect::Allow),
        map(tag_no_case("deny"), |_| Effect::Deny),
    ))(input)
}

pub fn action_parser(input: &str) -> Res<'_, Action> {
    let (input, action_str) = identifier(input)?;
    let action = match action_str.to_ascii_lowercase().as_str() {
        "read" => Action::Read,
        "write" => Action::Write,
        "delete" => Action::Delete,
        "execute" => Action::Execute,
        "subscribe" => Action::Subscribe,
        "publish" => Action::Publish,
        other => Action::Custom(other.to_string()),
    };

    Ok((input, action))
}

pub fn conditions_parser(input: &str) -> Res<'_, Vec<Condition>> {
    let (mut input, first_condition) = ws(condition_parser)(input)?;
    let mut conditions = vec![first_condition];

    loop {
        let (next_input, connector) = opt(ws(alt((tag_no_case("and"), tag_no_case("or")))))(input)?;
        if connector.is_none() {
            break;
        }
        let (next_input, condition) = ws(condition_parser)(next_input)?;
        conditions.push(condition);
        input = next_input;
    }

    Ok((input, conditions))
}

pub fn condition_parser(input: &str) -> Res<'_, Condition> {
    let (input, left) = ws(expression_parser)(input)?;
    let (input, operator) = ws(operator_parser)(input)?;
    let (input, right) = ws(expression_parser)(input)?;

    Ok((
        input,
        Condition {
            left,
            operator,
            right,
        },
    ))
}

pub fn operator_parser(input: &str) -> Res<'_, Operator> {
    alt((
        map(tag("=="), |_| Operator::Equal),
        map(tag("!="), |_| Operator::NotEqual),
        map(tag("<="), |_| Operator::LessThanOrEqual),
        map(tag(">="), |_| Operator::GreaterThanOrEqual),
        map(tag("<"), |_| Operator::LessThan),
        map(tag(">"), |_| Operator::GreaterThan),
        map(tag_no_case("in"), |_| Operator::In),
    ))(input)
}

pub fn expression_parser(input: &str) -> Res<'_, Expression> {
    alt((
        map(attribute_path_parser, Expression::AttributePath),
        map(string_literal_parser, Expression::StringLiteral),
        map(boolean_literal_parser, Expression::BooleanLiteral),
        map(number_literal_parser, Expression::NumberLiteral),
        map(list_literal_parser, Expression::ListLiteral),
    ))(input)
}

pub fn attribute_path_parser(input: &str) -> Res<'_, AttributePath> {
    let (input, (category_str, field_str)) =
        separated_pair(identifier, char('.'), identifier)(input)?;

    let category = match category_str.to_ascii_lowercase().as_str() {
        "subject" => AttributeCategory::Subject,
        "resource" => AttributeCategory::Resource,
        "environment" => AttributeCategory::Environment,
        "action" => AttributeCategory::Action,
        other => AttributeCategory::Custom(other.to_string()),
    };

    Ok((
        input,
        AttributePath {
            category,
            field: field_str.to_string(),
        },
    ))
}

pub fn string_literal_parser(input: &str) -> Res<'_, String> {
    map_res(
        recognize(delimited(
            char('"'),
            many0(alt((
                recognize(tuple((char('\\'), one_of(r#""\\/bfnrt"#)))),
                recognize(is_not("\\\"")),
            ))),
            char('"'),
        )),
        |raw: &str| serde_json::from_str::<String>(raw),
    )(input)
}

pub fn number_literal_parser(input: &str) -> Res<'_, f64> {
    map_res(
        recognize(tuple((
            opt(char('-')),
            digit1,
            opt(tuple((char('.'), cut(digit1)))),
        ))),
        |number_str: &str| -> Result<f64, ParseFloatError> { number_str.parse::<f64>() },
    )(input)
}

pub fn boolean_literal_parser(input: &str) -> Res<'_, bool> {
    alt((
        map(tag_no_case("true"), |_| true),
        map(tag_no_case("false"), |_| false),
    ))(input)
}

pub fn list_literal_parser(input: &str) -> Res<'_, Vec<Expression>> {
    delimited(
        ws(char('[')),
        separated_list0(ws(char(',')), ws(expression_parser)),
        ws(char(']')),
    )(input)
}

pub fn identifier(input: &str) -> Res<'_, &str> {
    recognize(tuple((
        take_while1(is_identifier_start),
        take_while(is_identifier_char),
    )))(input)
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '$')
}

pub fn ws<'a, F, O>(mut inner: F) -> impl FnMut(&'a str) -> Res<'a, O>
where
    F: FnMut(&'a str) -> Res<'a, O>,
{
    move |input: &'a str| {
        let (input, _) = multispace0(input)?;
        let (input, result) = inner(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, result))
    }
}

fn compute_location(input: &str, offset: usize) -> Option<(usize, usize)> {
    if offset > input.len() {
        return None;
    }

    let mut line = 1;
    let mut column = 1;

    for (idx, ch) in input.char_indices() {
        if idx == offset {
            return Some((line, column));
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    if offset == input.len() {
        Some((line, column))
    } else {
        None
    }
}
