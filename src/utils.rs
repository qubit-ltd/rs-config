/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Utility Functions
//!
//! Provides configuration-related utility functions, such as variable
//! substitution and JSON map construction for [`crate::Config::deserialize`].
//!
//! # Author
//!
//! Haixing Hu

use regex::Regex;
use serde_json::map::Entry;
use serde_json::{Map, Number, Value};
use std::sync::OnceLock;

use qubit_common::serde::duration_with_unit;
use qubit_value::{MultiValues, ValueError};

use super::{ConfigError, ConfigReader, ConfigResult, Property};

/// Regular expression pattern for variables
///
/// Matches variables in `${variable_name}` format
///
/// # Author
///
/// Haixing Hu
///
static VARIABLE_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Gets the regular expression pattern for variables
///
/// # Author
///
/// Haixing Hu
///
#[inline]
fn get_variable_pattern() -> &'static Regex {
    VARIABLE_PATTERN.get_or_init(|| {
        Regex::new(r"\$\{([^}]+)\}").expect("Failed to compile variable pattern regex")
    })
}

/// Maps a [`ValueError`] from typed property access to [`ConfigError`], using
/// `key` as the configuration path for type and conversion errors.
///
/// # Author
///
/// Haixing Hu
///
pub(crate) fn map_value_error(key: &str, err: ValueError) -> ConfigError {
    match err {
        ValueError::NoValue => ConfigError::PropertyHasNoValue(key.to_string()),
        ValueError::TypeMismatch { expected, actual } => {
            ConfigError::type_mismatch_at(key, expected, actual)
        }
        ValueError::ConversionFailed { from, to } => {
            ConfigError::conversion_error_at(key, format!("From {from} to {to}"))
        }
        ValueError::ConversionError(msg) => ConfigError::conversion_error_at(key, msg),
        ValueError::IndexOutOfBounds { index, len } => ConfigError::IndexOutOfBounds { index, len },
        ValueError::JsonSerializationError(msg) => {
            ConfigError::conversion_error_at(key, format!("JSON serialization error: {msg}"))
        }
        ValueError::JsonDeserializationError(msg) => {
            ConfigError::conversion_error_at(key, format!("JSON deserialization error: {msg}"))
        }
    }
}

/// Replaces variables in a string (`${name}`).
///
/// Used internally by [`crate::Config`] and [`crate::ConfigReader`] when
/// variable substitution is enabled.
///
/// # Author
///
/// Haixing Hu
///
pub(crate) fn substitute_variables<R: ConfigReader + ?Sized>(
    value: &str,
    config: &R,
    max_depth: usize,
) -> ConfigResult<String> {
    if value.is_empty() {
        return Ok(value.to_string());
    }

    let pattern = get_variable_pattern();
    let mut result = value.to_string();
    let mut depth = 0;

    loop {
        if !pattern.is_match(&result) {
            // No more variables to replace
            break;
        }

        if depth >= max_depth {
            return Err(ConfigError::SubstitutionDepthExceeded(max_depth));
        }

        // Replace all placeholders in a single regex pass.
        let mut first_error: Option<ConfigError> = None;
        let replaced = pattern.replace_all(&result, |caps: &regex::Captures| {
            let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            match find_variable_value(var_name, config) {
                Ok(v) => v,
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            }
        });
        if let Some(err) = first_error {
            return Err(err);
        }
        result = replaced.into_owned();

        depth += 1;
    }

    Ok(result)
}

/// Finds the value of a variable
///
/// First looks in the configuration. It falls back to environment variables
/// only when the key is missing or explicitly empty/null in config.
///
/// # Parameters
///
/// * `var_name` - Variable name
/// * `config` - Configuration object
///
/// # Returns
///
/// Returns the variable value on success, or an error on failure
///
/// # Author
///
/// Haixing Hu
///
fn find_variable_value<R: ConfigReader + ?Sized>(
    var_name: &str,
    config: &R,
) -> ConfigResult<String> {
    // 1. Try configuration first.
    match config.get::<String>(var_name) {
        Ok(value) => Ok(value),
        // Only missing or empty values can fall back to env vars.
        Err(ConfigError::PropertyNotFound(_)) | Err(ConfigError::PropertyHasNoValue(_)) => {
            std::env::var(var_name).map_err(|_| {
                ConfigError::SubstitutionError(format!("Cannot resolve variable: {}", var_name))
            })
        }
        // Type/conversion errors in config should surface directly instead of
        // being silently masked by environment values.
        Err(err) => Err(err),
    }
}

/// Inserts a value into the serde object used by [`crate::Config::deserialize`].
///
/// Keys containing dots are interpreted as nested object paths (for example,
/// `db.host` becomes `{ "db": { "host": ... } }`). If path insertion
/// conflicts with an existing scalar/object shape, this function falls back to
/// the original flat-key behavior (`"db.host"` as a single key) for backward
/// compatibility.
pub(crate) fn insert_deserialize_value(root: &mut Map<String, Value>, key: &str, value: Value) {
    if !key.contains('.') || key.is_empty() {
        root.insert(key.to_string(), value);
        return;
    }

    let fallback_value = value.clone();
    if try_insert_nested_json_value(root, key, value).is_err() {
        root.insert(key.to_string(), fallback_value);
    }
}

/// Tries to insert a dotted key as a nested JSON object path.
///
/// Returns `Err(())` when the key is malformed (`a..b`, `.a`, `a.`) or when an
/// insertion path conflicts with an existing non-object leaf.
fn try_insert_nested_json_value(
    root: &mut Map<String, Value>,
    key: &str,
    value: Value,
) -> Result<(), ()> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(());
    }
    let (leaf, parents) = parts
        .split_last()
        .expect("split on a string always returns at least one segment");

    let mut current = root;
    for part in parents {
        let next = match current.entry(part.to_string()) {
            Entry::Vacant(entry) => entry.insert(Value::Object(Map::new())),
            Entry::Occupied(entry) => entry.into_mut(),
        };

        match next {
            Value::Object(obj) => {
                current = obj;
            }
            _ => return Err(()),
        }
    }

    match current.entry((*leaf).to_string()) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(_) => Err(()),
    }
}

/// Converts a [`Property`] into [`serde_json::Value`] (for
/// [`crate::Config::deserialize`]).
///
/// # Parameters
///
/// * `prop` - Source property.
///
/// # Returns
///
/// JSON null, scalar, array, or object matching the stored [`MultiValues`].
pub(crate) fn property_to_json_value(prop: &Property) -> Value {
    let mv = prop.value();

    match mv {
        MultiValues::Empty(_) => Value::Null,
        MultiValues::Bool(v) => {
            if v.len() == 1 {
                Value::Bool(v[0])
            } else {
                Value::Array(v.iter().map(|b| Value::Bool(*b)).collect())
            }
        }
        MultiValues::Int8(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::Int16(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::Int32(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::Int64(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::IntSize(v) => scalar_or_array(v, |x| Value::Number(Number::from(*x as i64))),
        MultiValues::UInt8(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::UInt16(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::UInt32(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::UInt64(v) => scalar_or_array(v, |x| Value::Number((*x).into())),
        MultiValues::UIntSize(v) => scalar_or_array(v, |x| Value::Number(Number::from(*x as u64))),
        MultiValues::Float32(v) => scalar_or_array(v, |x| {
            Number::from_f64(*x as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }),
        MultiValues::Float64(v) => scalar_or_array(v, |x| {
            Number::from_f64(*x)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }),
        MultiValues::String(v) => scalar_or_array(v, |x| Value::String(x.clone())),
        MultiValues::Duration(v) => {
            scalar_or_array(v, |x| Value::String(duration_with_unit::format(x)))
        }
        MultiValues::Url(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::StringMap(v) => {
            if v.len() == 1 {
                let obj: Map<String, Value> = v[0]
                    .iter()
                    .map(|(k, val)| (k.clone(), Value::String(val.clone())))
                    .collect();
                Value::Object(obj)
            } else {
                Value::Array(
                    v.iter()
                        .map(|m| {
                            let obj: Map<String, Value> = m
                                .iter()
                                .map(|(k, val)| (k.clone(), Value::String(val.clone())))
                                .collect();
                            Value::Object(obj)
                        })
                        .collect(),
                )
            }
        }
        MultiValues::Json(v) => {
            if v.len() == 1 {
                v[0].clone()
            } else {
                Value::Array(v.clone())
            }
        }
        MultiValues::Char(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::BigInteger(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::BigDecimal(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::DateTime(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::Date(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::Time(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::Instant(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::Int128(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
        MultiValues::UInt128(v) => scalar_or_array(v, |x| Value::String(x.to_string())),
    }
}

/// If `v` has one element, returns `f(&v[0])`; otherwise a JSON array of `f`
/// applied to each item.
///
/// # Parameters
///
/// * `v` - Multi-values slice from a [`Property`].
/// * `f` - Maps each element to [`serde_json::Value`].
///
/// # Returns
///
/// A scalar or array [`serde_json::Value`].
fn scalar_or_array<T, F>(v: &[T], f: F) -> Value
where
    F: Fn(&T) -> Value,
{
    if v.len() == 1 {
        f(&v[0])
    } else {
        Value::Array(v.iter().map(f).collect())
    }
}

#[cfg(test)]
mod substitute_variable_tests {
    use super::{insert_deserialize_value, map_value_error, substitute_variables};
    use crate::{Config, ConfigError};
    use qubit_common::DataType;
    use qubit_value::ValueError;
    use serde_json::{Map, json};

    #[test]
    fn test_map_value_error_additional_variants() {
        let conversion_failed = map_value_error(
            "port",
            ValueError::ConversionFailed {
                from: DataType::String,
                to: DataType::Int32,
            },
        );
        assert!(matches!(
            conversion_failed,
            ConfigError::ConversionError { ref key, ref message }
                if key == "port" && message.contains("From")
        ));

        let conversion_error =
            map_value_error("port", ValueError::ConversionError("bad int".to_string()));
        assert!(matches!(
            conversion_error,
            ConfigError::ConversionError { ref key, ref message }
                if key == "port" && message == "bad int"
        ));

        let out_of_bounds =
            map_value_error("items", ValueError::IndexOutOfBounds { index: 3, len: 2 });
        assert!(matches!(
            out_of_bounds,
            ConfigError::IndexOutOfBounds { index: 3, len: 2 }
        ));

        let json_serialization = map_value_error(
            "payload",
            ValueError::JsonSerializationError("serializer failed".to_string()),
        );
        assert!(matches!(
            json_serialization,
            ConfigError::ConversionError { ref key, ref message }
                if key == "payload" && message.contains("JSON serialization error")
        ));

        let json_deserialization = map_value_error(
            "payload",
            ValueError::JsonDeserializationError("parser failed".to_string()),
        );
        assert!(matches!(
            json_deserialization,
            ConfigError::ConversionError { ref key, ref message }
                if key == "payload" && message.contains("JSON deserialization error")
        ));
    }

    #[test]
    fn test_insert_deserialize_value_fallback_branches() {
        let mut malformed = Map::new();
        insert_deserialize_value(&mut malformed, "bad..key", json!("value"));
        assert_eq!(malformed.get("bad..key"), Some(&json!("value")));

        let mut occupied_leaf = Map::new();
        insert_deserialize_value(&mut occupied_leaf, "a.b", json!(1));
        insert_deserialize_value(&mut occupied_leaf, "a.b", json!(2));
        assert_eq!(occupied_leaf.get("a.b"), Some(&json!(2)));

        let mut scalar_parent = Map::new();
        insert_deserialize_value(&mut scalar_parent, "a", json!(1));
        insert_deserialize_value(&mut scalar_parent, "a.b", json!(2));
        assert_eq!(scalar_parent.get("a.b"), Some(&json!(2)));
    }

    #[test]
    fn test_substitute_simple() {
        let mut config = Config::new();
        config.set("name", "world").unwrap();

        let result = substitute_variables("Hello, ${name}!", &config, 10).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_substitute_multiple() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", "8080").unwrap();

        let result = substitute_variables("http://${host}:${port}/api", &config, 10).unwrap();
        assert_eq!(result, "http://localhost:8080/api");
    }

    #[test]
    fn test_substitute_repeated_placeholder() {
        let mut config = Config::new();
        config.set("name", "world").unwrap();

        let result = substitute_variables("${name}-${name}-${name}", &config, 10).unwrap();
        assert_eq!(result, "world-world-world");
    }

    #[test]
    fn test_substitute_recursive() {
        let mut config = Config::new();
        config.set("a", "value_a").unwrap();
        config.set("b", "${a}_b").unwrap();
        config.set("c", "${b}_c").unwrap();

        let result = substitute_variables("${c}", &config, 10).unwrap();
        assert_eq!(result, "value_a_b_c");
    }

    #[test]
    fn test_substitute_depth_exceeded() {
        let mut config = Config::new();
        config.set("a", "${b}").unwrap();
        config.set("b", "${a}").unwrap();

        let result = substitute_variables("${a}", &config, 5);
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded(5))
        ));
    }

    #[test]
    fn test_substitute_env_var() {
        unsafe {
            std::env::set_var("TEST_VAR", "test_value");
        }

        let config = Config::new();
        let result = substitute_variables("Value: ${TEST_VAR}", &config, 10).unwrap();
        assert_eq!(result, "Value: test_value");

        unsafe {
            std::env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_substitute_empty_string() {
        let config = Config::new();
        let result = substitute_variables("", &config, 10).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_substitute_zero_depth_without_placeholders_should_succeed() {
        let config = Config::new();
        let result = substitute_variables("plain text", &config, 0).unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_substitute_variable_not_found() {
        let config = Config::new();
        let result = substitute_variables("${NONEXISTENT_VAR}", &config, 10);
        assert!(matches!(result, Err(ConfigError::SubstitutionError(_))));

        let err = result.expect_err("unresolved variable should return an error");
        assert!(err.to_string().contains("NONEXISTENT_VAR"));
    }

    #[test]
    fn test_substitute_no_variables() {
        let config = Config::new();
        let result = substitute_variables("Plain text with no variables", &config, 10).unwrap();
        assert_eq!(result, "Plain text with no variables");
    }

    #[test]
    fn test_substitute_mixed_sources() {
        unsafe {
            std::env::set_var("ENV_VAR", "from_env");
        }

        let mut config = Config::new();
        config.set("CONFIG_VAR", "from_config").unwrap();

        let result = substitute_variables("${CONFIG_VAR} and ${ENV_VAR}", &config, 10).unwrap();
        assert_eq!(result, "from_config and from_env");

        unsafe {
            std::env::remove_var("ENV_VAR");
        }
    }

    #[test]
    fn test_substitute_config_priority_over_env() {
        unsafe {
            std::env::set_var("SHARED_VAR", "from_env");
        }

        let mut config = Config::new();
        config.set("SHARED_VAR", "from_config").unwrap();

        let result = substitute_variables("${SHARED_VAR}", &config, 10).unwrap();
        assert_eq!(result, "from_config");

        unsafe {
            std::env::remove_var("SHARED_VAR");
        }
    }

    #[test]
    fn test_substitute_does_not_fallback_to_env_on_config_type_error() {
        unsafe {
            std::env::set_var("STRICT_VAR", "from_env");
        }

        let mut config = Config::new();
        config.set("STRICT_VAR", 8080i32).unwrap();

        let result = substitute_variables("${STRICT_VAR}", &config, 10);
        assert!(matches!(
            result,
            Err(ConfigError::TypeMismatch { .. }) | Err(ConfigError::ConversionError { .. })
        ));

        unsafe {
            std::env::remove_var("STRICT_VAR");
        }
    }
}
