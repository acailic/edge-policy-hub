use serde_json::Value;
use tracing::{debug, warn};

use super::{TransformDirective, TransformError, MAX_TRANSFORM_DEPTH, REDACTED_PLACEHOLDER};

pub struct PayloadTransformer;

impl PayloadTransformer {
    pub fn new() -> Self {
        Self
    }

    pub fn transform_payload(
        &self,
        payload: &[u8],
        directives: &[TransformDirective],
    ) -> Result<Vec<u8>, TransformError> {
        // Try parsing payload as JSON
        let mut json_value: Value = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(e) => {
                warn!("Payload is not valid JSON, returning unchanged: {}", e);
                return Ok(payload.to_vec());
            }
        };

        let mut total_changes = 0;

        // Apply each transformation directive
        for directive in directives {
            let changes = match directive {
                TransformDirective::RemoveFields(paths) => {
                    self.remove_fields_by_path(&mut json_value, paths)?
                }
                TransformDirective::RedactFields(paths) => {
                    self.redact_fields_by_path(&mut json_value, paths)?
                }
                TransformDirective::StripCoordinates => {
                    self.strip_gps_coordinates(&mut json_value)?
                }
            };
            total_changes += changes;
        }

        if total_changes > 0 {
            debug!(
                "Applied {} field transformations to payload",
                total_changes
            );
        }

        // Serialize modified JSON back to bytes
        let transformed = serde_json::to_vec(&json_value)?;
        Ok(transformed)
    }

    fn remove_fields_by_path(
        &self,
        value: &mut Value,
        paths: &[String],
    ) -> Result<usize, TransformError> {
        let mut removed_count = 0;

        for path in paths {
            removed_count += self.remove_path(value, path, 0)?;
        }

        Ok(removed_count)
    }

    fn remove_path(
        &self,
        value: &mut Value,
        path: &str,
        depth: usize,
    ) -> Result<usize, TransformError> {
        if depth > MAX_TRANSFORM_DEPTH {
            return Err(TransformError::MaxDepthExceeded);
        }

        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(0);
        }

        if parts.len() == 1 {
            // Final key to remove
            match value {
                Value::Object(map) => {
                    if map.remove(parts[0]).is_some() {
                        debug!("Removed field: {}", path);
                        return Ok(1);
                    }
                }
                Value::Array(arr) => {
                    // Remove from all array elements
                    let mut count = 0;
                    for item in arr.iter_mut() {
                        if let Value::Object(map) = item {
                            if map.remove(parts[0]).is_some() {
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        debug!("Removed field '{}' from {} array elements", path, count);
                        return Ok(count);
                    }
                }
                _ => {}
            }
            return Ok(0);
        }

        // Navigate to parent and recurse
        let first = parts[0];
        let rest = parts[1..].join(".");

        match value {
            Value::Object(map) => {
                if let Some(child) = map.get_mut(first) {
                    return self.remove_path(child, &rest, depth + 1);
                }
            }
            Value::Array(arr) => {
                let mut count = 0;
                for item in arr.iter_mut() {
                    count += self.remove_path(item, path, depth + 1)?;
                }
                return Ok(count);
            }
            _ => {}
        }

        Ok(0)
    }

    fn redact_fields_by_path(
        &self,
        value: &mut Value,
        paths: &[String],
    ) -> Result<usize, TransformError> {
        let mut redacted_count = 0;

        for path in paths {
            redacted_count += self.redact_path(value, path, 0)?;
        }

        Ok(redacted_count)
    }

    fn redact_path(
        &self,
        value: &mut Value,
        path: &str,
        depth: usize,
    ) -> Result<usize, TransformError> {
        if depth > MAX_TRANSFORM_DEPTH {
            return Err(TransformError::MaxDepthExceeded);
        }

        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(0);
        }

        if parts.len() == 1 {
            // Final key to redact
            match value {
                Value::Object(map) => {
                    if map.contains_key(parts[0]) {
                        map.insert(
                            parts[0].to_string(),
                            Value::String(REDACTED_PLACEHOLDER.to_string()),
                        );
                        debug!("Redacted field: {}", path);
                        return Ok(1);
                    }
                }
                Value::Array(arr) => {
                    let mut count = 0;
                    for item in arr.iter_mut() {
                        if let Value::Object(map) = item {
                            if map.contains_key(parts[0]) {
                                map.insert(
                                    parts[0].to_string(),
                                    Value::String(REDACTED_PLACEHOLDER.to_string()),
                                );
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        debug!("Redacted field '{}' in {} array elements", path, count);
                        return Ok(count);
                    }
                }
                _ => {}
            }
            return Ok(0);
        }

        // Navigate to parent and recurse
        let first = parts[0];
        let rest = parts[1..].join(".");

        match value {
            Value::Object(map) => {
                if let Some(child) = map.get_mut(first) {
                    return self.redact_path(child, &rest, depth + 1);
                }
            }
            Value::Array(arr) => {
                let mut count = 0;
                for item in arr.iter_mut() {
                    count += self.redact_path(item, path, depth + 1)?;
                }
                return Ok(count);
            }
            _ => {}
        }

        Ok(0)
    }

    fn strip_gps_coordinates(&self, value: &mut Value) -> Result<usize, TransformError> {
        self.strip_gps_recursive(value, 0)
    }

    fn strip_gps_recursive(&self, value: &mut Value, depth: usize) -> Result<usize, TransformError> {
        if depth > MAX_TRANSFORM_DEPTH {
            return Err(TransformError::MaxDepthExceeded);
        }

        let mut stripped_count = 0;

        match value {
            Value::Object(map) => {
                // List of leaf GPS coordinate field names (not container objects)
                let gps_coordinate_fields = [
                    "latitude",
                    "longitude",
                    "lat",
                    "lon",
                    "lng",
                    "gps",
                    "coordinates",
                ];

                // Remove only the coordinate fields from this level
                for field in &gps_coordinate_fields {
                    if map.remove(*field).is_some() {
                        stripped_count += 1;
                        debug!("Stripped GPS coordinate field: {}", field);
                    }
                }

                // Recurse into all remaining fields (including "location", "position", etc.)
                // This preserves container objects but strips coordinates inside them
                for (field_name, child_value) in map.iter_mut() {
                    // Special handling for location/position objects: recurse to strip coords inside
                    if field_name == "location" || field_name == "position" {
                        debug!("Recursing into '{}' object to strip coordinates while preserving other fields", field_name);
                    }
                    stripped_count += self.strip_gps_recursive(child_value, depth + 1)?;
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    stripped_count += self.strip_gps_recursive(item, depth + 1)?;
                }
            }
            _ => {}
        }

        Ok(stripped_count)
    }
}

impl Default for PayloadTransformer {
    fn default() -> Self {
        Self::new()
    }
}
