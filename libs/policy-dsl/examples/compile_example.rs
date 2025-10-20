use edge_policy_dsl::{compile_policy, PolicyDslError, PolicyMetadata};

fn main() {
    let success_source = r#"
# Example policy that enforces EU-only access
allow read sensor_data if subject.tenant_id == "tenant-eu" and subject.device_location in ["DE", "FR"] and resource.region == "EU"
"#;

    let success_metadata = PolicyMetadata {
        version: "2.1.0".to_string(),
        author: Some("operator@example.com".to_string()),
        description: Some("EU residency access guardrail".to_string()),
        ..PolicyMetadata::default()
    };

    match compile_policy(success_source, "tenant-eu", Some(success_metadata)) {
        Ok(compiled) => {
            println!("Compiled policy name: {}", compiled.name);
            println!("Rego preview:\n{}\n", compiled.rego);
        }
        Err(err) => {
            println!("Compilation unexpectedly failed: {err}");
        }
    }

    let parse_error_source =
        r#"allow read sensor_data if subject.tenant_id = "tenant-eu" and resource.region == "EU""#;

    match compile_policy(parse_error_source, "tenant-eu", None) {
        Ok(_) => println!("Parser error example unexpectedly compiled"),
        Err(PolicyDslError::ParseError { message, location }) => {
            if let Some((line, col)) = location {
                println!("Parse error detected at line {line}, column {col}: {message}");
            } else {
                println!("Parse error detected: {message}");
            }
        }
        Err(err) => println!("Received different error: {err}"),
    }

    let invalid_attribute_source =
        r#"allow read sensor_data if subject.clearanceLevel >= 3 and action == "read""#;

    match compile_policy(invalid_attribute_source, "tenant-eu", None) {
        Ok(_) => println!("Invalid attribute example unexpectedly compiled"),
        Err(PolicyDslError::InvalidAttribute { path, reason }) => {
            println!("Invalid attribute `{path}`: {reason}");
        }
        Err(err) => println!("Unexpected error: {err}"),
    }
}
