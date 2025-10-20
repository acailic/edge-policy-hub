use super::{RedactionError, MAX_REDACTION_DEPTH};
use serde_json::Value;
use tracing::{debug, info};

pub struct RedactionEngine;

impl RedactionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn redact_fields(
        &self,
        json_body: &[u8],
        paths: &[String],
    ) -> Result<Vec<u8>, RedactionError> {
        // Try to parse as JSON
        let mut value: Value = match serde_json::from_slice(json_body) {
            Ok(v) => v,
            Err(_) => {
                debug!("Response body is not valid JSON, skipping redaction");
                return Ok(json_body.to_vec());
            }
        };

        let mut fields_removed = 0;

        // Apply each redaction path
        for path in paths {
            if Self::remove_field_by_path(&mut value, path) {
                fields_removed += 1;
                debug!(path = %path, "Removed field");
            }
        }

        info!(fields_removed = fields_removed, "Redaction completed");

        // Serialize back to JSON
        let redacted_bytes = serde_json::to_vec(&value)?;
        Ok(redacted_bytes)
    }

    fn remove_field_by_path(value: &mut Value, path: &str) -> bool {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return false;
        }

        // Try to match the path starting from current level
        if Self::remove_field_recursive(value, &parts, 0) {
            return true;
        }

        // If not matched at current level, try matching at any nested level (depth-first search)
        Self::remove_field_at_any_depth(value, &parts, 0)
    }

    fn remove_field_recursive(value: &mut Value, path_parts: &[&str], depth: usize) -> bool {
        if depth > MAX_REDACTION_DEPTH {
            return false;
        }

        if path_parts.is_empty() {
            return false;
        }

        let current_key = path_parts[0];
        let remaining_parts = &path_parts[1..];

        match value {
            Value::Object(map) => {
                if remaining_parts.is_empty() {
                    // This is the final key to remove
                    map.remove(current_key).is_some()
                } else {
                    // Navigate deeper following the exact path
                    if let Some(nested_value) = map.get_mut(current_key) {
                        Self::remove_field_recursive(nested_value, remaining_parts, depth + 1)
                    } else {
                        false
                    }
                }
            }
            Value::Array(arr) => {
                // Apply redaction to all array elements
                let mut any_removed = false;
                for item in arr.iter_mut() {
                    if Self::remove_field_recursive(item, path_parts, depth + 1) {
                        any_removed = true;
                    }
                }
                any_removed
            }
            _ => false,
        }
    }

    /// Try to match the path at any depth in the JSON structure
    fn remove_field_at_any_depth(value: &mut Value, path_parts: &[&str], depth: usize) -> bool {
        if depth > MAX_REDACTION_DEPTH {
            return false;
        }

        let mut any_removed = false;

        match value {
            Value::Object(map) => {
                // Try to match path starting from each nested object
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(nested_value) = map.get_mut(&key) {
                        // Try exact match from this point
                        if Self::remove_field_recursive(nested_value, path_parts, depth + 1) {
                            any_removed = true;
                        } else {
                            // Continue searching deeper
                            if Self::remove_field_at_any_depth(nested_value, path_parts, depth + 1)
                            {
                                any_removed = true;
                            }
                        }
                    }
                }
            }
            Value::Array(arr) => {
                // Search in array elements
                for item in arr.iter_mut() {
                    if Self::remove_field_at_any_depth(item, path_parts, depth + 1) {
                        any_removed = true;
                    }
                }
            }
            _ => {}
        }

        any_removed
    }

    #[allow(dead_code)]
    fn remove_field_by_name(value: &mut Value, field_name: &str, depth: usize) -> usize {
        if depth > MAX_REDACTION_DEPTH {
            return 0;
        }

        let mut count = 0;

        match value {
            Value::Object(map) => {
                // Remove the field if it exists
                if map.remove(field_name).is_some() {
                    count += 1;
                }

                // Recurse into nested objects and arrays
                for (_, nested_value) in map.iter_mut() {
                    count += Self::remove_field_by_name(nested_value, field_name, depth + 1);
                }
            }
            Value::Array(arr) => {
                // Recurse into array elements
                for item in arr.iter_mut() {
                    count += Self::remove_field_by_name(item, field_name, depth + 1);
                }
            }
            _ => {}
        }

        count
    }
}

impl Default for RedactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_redact_simple_field() {
        let engine = RedactionEngine::new();
        let body = json!({
            "name": "Alice",
            "email": "alice@example.com",
            "age": 30
        });

        let body_bytes = serde_json::to_vec(&body).unwrap();
        let paths = vec!["email".to_string()];

        let result = engine.redact_fields(&body_bytes, &paths).unwrap();
        let redacted: Value = serde_json::from_slice(&result).unwrap();

        assert!(!redacted.get("email").is_some());
        assert_eq!(redacted.get("name").unwrap(), "Alice");
        assert_eq!(redacted.get("age").unwrap(), 30);
    }

    #[test]
    fn test_redact_nested_field() {
        let engine = RedactionEngine::new();
        let body = json!({
            "user": {
                "name": "Alice",
                "pii": {
                    "email": "alice@example.com",
                    "phone": "+1234567890"
                }
            }
        });

        let body_bytes = serde_json::to_vec(&body).unwrap();
        let paths = vec!["pii.email".to_string()];

        let result = engine.redact_fields(&body_bytes, &paths).unwrap();
        let redacted: Value = serde_json::from_slice(&result).unwrap();

        let pii = redacted.get("user").unwrap().get("pii").unwrap();
        assert!(!pii.get("email").is_some());
        assert_eq!(pii.get("phone").unwrap(), "+1234567890");
    }

    #[test]
    fn test_redact_multiple_fields() {
        let engine = RedactionEngine::new();
        let body = json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com",
                "phone": "+1234567890",
                "age": 30
            }
        });

        let body_bytes = serde_json::to_vec(&body).unwrap();
        let paths = vec!["email".to_string(), "phone".to_string()];

        let result = engine.redact_fields(&body_bytes, &paths).unwrap();
        let redacted: Value = serde_json::from_slice(&result).unwrap();

        let user = redacted.get("user").unwrap();
        assert!(!user.get("email").is_some());
        assert!(!user.get("phone").is_some());
        assert_eq!(user.get("name").unwrap(), "Alice");
        assert_eq!(user.get("age").unwrap(), 30);
    }

    #[test]
    fn test_redact_array_elements() {
        let engine = RedactionEngine::new();
        let body = json!({
            "users": [
                {"name": "Alice", "email": "alice@example.com"},
                {"name": "Bob", "email": "bob@example.com"}
            ]
        });

        let body_bytes = serde_json::to_vec(&body).unwrap();
        let paths = vec!["email".to_string()];

        let result = engine.redact_fields(&body_bytes, &paths).unwrap();
        let redacted: Value = serde_json::from_slice(&result).unwrap();

        let users = redacted.get("users").unwrap().as_array().unwrap();
        assert!(!users[0].get("email").is_some());
        assert!(!users[1].get("email").is_some());
        assert_eq!(users[0].get("name").unwrap(), "Alice");
        assert_eq!(users[1].get("name").unwrap(), "Bob");
    }

    #[test]
    fn test_non_json_passthrough() {
        let engine = RedactionEngine::new();
        let body = b"This is not JSON";
        let paths = vec!["email".to_string()];

        let result = engine.redact_fields(body, &paths).unwrap();
        assert_eq!(result, body);
    }
}
