// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde_json::Map;
use serde_json::Value;
use serde_json::map::Entry;

use super::interpolation::substitute_variables_with_fallback;
use super::map_value_error;
use crate::ConfigError;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::Property;

/// Inserts a value into the serde object used by
/// [`crate::Config::deserialize`].
pub(crate) fn insert_deserialize_value(
    root: &mut Map<String, Value>,
    key: &str,
    value: Value,
) -> ConfigResult<()> {
    if !key.contains('.') || key.is_empty() {
        root.insert(key.to_string(), value);
        return Ok(());
    }

    try_insert_nested_json_value(root, key, value)
}

/// Tries to insert a dotted key as a nested JSON object path.
fn try_insert_nested_json_value(
    root: &mut Map<String, Value>,
    key: &str,
    value: Value,
) -> ConfigResult<()> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(ConfigError::KeyConflict {
            source_id: None,
            path: key.to_string(),
            existing: "valid dotted key path".to_string(),
            incoming: "malformed dotted key path".to_string(),
        });
    }
    let (leaf, parents) = parts
        .split_last()
        .expect("split on a string always returns at least one segment");

    let mut current = root;
    let mut path = String::new();
    for part in parents {
        if path.is_empty() {
            path.push_str(part);
        } else {
            path.push('.');
            path.push_str(part);
        }
        let next = match current.entry(part.to_string()) {
            Entry::Vacant(entry) => entry.insert(Value::Object(Map::new())),
            Entry::Occupied(entry) => entry.into_mut(),
        };

        match next {
            Value::Object(obj) => {
                current = obj;
            }
            other => {
                return Err(ConfigError::KeyConflict {
                    source_id: None,
                    path,
                    existing: json_value_kind(other).to_string(),
                    incoming: format!("object required by dotted key '{key}'"),
                });
            }
        }
    }

    if let Some(existing) = current.get(*leaf)
        && existing.is_object() != value.is_object()
    {
        let path = if parents.is_empty() {
            (*leaf).to_string()
        } else {
            format!("{}.{}", parents.join("."), leaf)
        };
        return Err(ConfigError::KeyConflict {
            source_id: None,
            path,
            existing: json_value_kind(existing).to_string(),
            incoming: json_value_kind(&value).to_string(),
        });
    }

    current.insert((*leaf).to_string(), value);
    Ok(())
}

/// Returns a short diagnostic name for a JSON value kind.
fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Converts a property to its JSON representation and optionally interpolates
/// every prepared string leaf exactly once.
///
/// # Parameters
///
/// * `prop` - Property to project into the structured Serde representation.
/// * `path` - Root-relative path used for conversion and interpolation errors.
/// * `primary` - Reader used first for placeholder lookup.
/// * `fallback` - Reader used when a placeholder is absent from `primary`.
/// * `interpolate` - Whether prepared JSON string leaves are interpolated.
///
/// # Returns
///
/// The prepared JSON value.
///
/// # Errors
///
/// Returns conversion, interpolation, or interpolation-limit errors with
/// `path` context.
pub(crate) fn prepare_deserialize_value<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    prop: &Property,
    path: &str,
    primary: &P,
    fallback: &F,
    interpolate: bool,
) -> ConfigResult<Value> {
    let mut value = prop
        .value()
        .to_json_value_with(
            primary.read_policy().conversion_policy(),
            primary.read_policy().conversion_limits(),
        )
        .map_err(|error| map_value_error(path, error))?;
    if interpolate {
        substitute_json_strings_with_fallback(&mut value, path, primary, fallback)?;
    }
    Ok(value)
}

/// Applies variable substitution to every JSON string leaf with fallback scope.
fn substitute_json_strings_with_fallback<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    value: &mut Value,
    path: &str,
    primary: &P,
    fallback: &F,
) -> ConfigResult<()> {
    let options = primary.read_policy();

    match value {
        Value::String(s) => {
            *s = substitute_variables_with_fallback(s, primary, fallback, options, path)?;
        }
        Value::Array(values) => {
            for value in values {
                substitute_json_strings_with_fallback(value, path, primary, fallback)?;
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                substitute_json_strings_with_fallback(value, path, primary, fallback)?;
            }
        }
        _ => {}
    }

    Ok(())
}
