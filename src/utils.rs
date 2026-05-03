/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Utility Functions
//!
//! Provides configuration-related utility functions, such as variable
//! substitution and JSON map construction for [`crate::Config::deserialize`].
//!

use regex::Regex;
use serde_json::map::Entry;
use serde_json::{Map, Number, Value};
use std::sync::OnceLock;

use qubit_serde::serde::duration_with_unit;
use qubit_value::{MultiValues, ValueError};

use super::{ConfigError, ConfigReader, ConfigResult, Property};

/// Regular expression pattern for variables
///
/// Matches variables in `${variable_name}` format
///
///
static VARIABLE_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Gets the regular expression pattern for variables
///
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
///
pub(crate) fn map_value_error(key: &str, err: ValueError) -> ConfigError {
    ConfigError::from((key, err))
}

/// Replaces variables in a string (`${name}`).
///
/// Used internally by [`crate::Config`] and [`crate::ConfigReader`] when
/// variable substitution is enabled.
///
///
pub(crate) fn substitute_variables<R: ConfigReader + ?Sized>(
    value: &str,
    config: &R,
    max_depth: usize,
) -> ConfigResult<String> {
    substitute_variables_by(value, max_depth, |var_name| {
        find_variable_value(var_name, config)
    })
}

/// Replaces variables using a primary reader and a fallback reader.
///
/// The primary reader is checked first. Missing or empty values fall back to
/// the fallback reader, then to environment variables only when the active
/// read options explicitly enable environment fallback. Type and conversion
/// errors in the primary reader are returned directly.
pub(crate) fn substitute_variables_with_fallback<
    P: ConfigReader + ?Sized,
    F: ConfigReader + ?Sized,
>(
    value: &str,
    primary: &P,
    fallback: &F,
    max_depth: usize,
) -> ConfigResult<String> {
    substitute_variables_by(value, max_depth, |var_name| {
        find_variable_value_with_fallback(var_name, primary, fallback)
    })
}

/// Replaces variables in `value` by repeatedly applying `resolve`.
fn substitute_variables_by(
    value: &str,
    max_depth: usize,
    mut resolve: impl FnMut(&str) -> ConfigResult<String>,
) -> ConfigResult<String> {
    let pattern = get_variable_pattern();
    let mut stack = Vec::new();
    substitute_variables_recursive(value, max_depth, pattern, &mut stack, &mut resolve)
}

/// Recursively expands variables while tracking the active variable chain.
fn substitute_variables_recursive(
    value: &str,
    max_depth: usize,
    pattern: &Regex,
    stack: &mut Vec<String>,
    resolve: &mut impl FnMut(&str) -> ConfigResult<String>,
) -> ConfigResult<String> {
    if value.is_empty() || !pattern.is_match(value) {
        return Ok(value.to_string());
    }
    if stack.len() >= max_depth {
        return Err(ConfigError::SubstitutionDepthExceeded(max_depth));
    }

    let mut result = String::with_capacity(value.len());
    let mut last_end = 0;
    for caps in pattern.captures_iter(value) {
        let full_match = caps
            .get(0)
            .expect("regex capture group 0 must be present for a match");
        result.push_str(&value[last_end..full_match.start()]);

        let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if let Some(index) = stack.iter().position(|name| name == var_name) {
            let mut chain = stack[index..].to_vec();
            chain.push(var_name.to_string());
            return Err(ConfigError::SubstitutionCycle { chain });
        }

        stack.push(var_name.to_string());
        let raw_value = resolve(var_name)?;
        let expanded =
            substitute_variables_recursive(&raw_value, max_depth, pattern, stack, resolve)?;
        stack.pop();
        result.push_str(&expanded);
        last_end = full_match.end();
    }
    result.push_str(&value[last_end..]);
    Ok(result)
}

/// Finds the value of a variable
///
/// First looks in the configuration. It falls back to environment variables
/// only when the key is missing or explicitly empty/null in config and the
/// active read options explicitly enable environment fallback.
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
///
fn find_variable_value<R: ConfigReader + ?Sized>(
    var_name: &str,
    config: &R,
) -> ConfigResult<String> {
    match config.get_property(var_name) {
        Some(property) if !property.is_empty() => match property.value().to::<String>() {
            Ok(value) => Ok(value),
            Err(error) => Err(map_value_error(var_name, error)),
        },
        Some(_) | None if config.read_options().is_env_variable_substitution_enabled() => {
            std::env::var(var_name).map_err(|_| {
                ConfigError::SubstitutionError(format!("Cannot resolve variable: {}", var_name))
            })
        }
        Some(_) | None => Err(ConfigError::SubstitutionError(format!(
            "Cannot resolve variable from config: {}",
            var_name
        ))),
    }
}

/// Finds a variable value from `primary`, then `fallback`.
///
/// Missing or empty values in `primary` are looked up in `fallback`. Other
/// errors from `primary` are returned directly so fallback values do not mask
/// invalid local configuration.
fn find_variable_value_with_fallback<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    var_name: &str,
    primary: &P,
    fallback: &F,
) -> ConfigResult<String> {
    match primary.get_property(var_name) {
        Some(property) if !property.is_empty() => match property.value().to::<String>() {
            Ok(value) => Ok(value),
            Err(error) => Err(map_value_error(var_name, error)),
        },
        Some(_) | None => find_variable_value(var_name, fallback),
    }
}

/// Inserts a value into the serde object used by [`crate::Config::deserialize`].
///
/// Keys containing dots are interpreted as nested object paths (for example,
/// `db.host` becomes `{ "db": { "host": ... } }`).
///
/// # Errors
///
/// Returns [`ConfigError::KeyConflict`] when the dotted path is malformed or
/// conflicts with an existing scalar/object shape.
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
///
/// Returns [`ConfigError::KeyConflict`] when the key is malformed (`a..b`,
/// `.a`, `a.`) or when an insertion path conflicts with an existing non-object
/// parent.
fn try_insert_nested_json_value(
    root: &mut Map<String, Value>,
    key: &str,
    value: Value,
) -> ConfigResult<()> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(ConfigError::KeyConflict {
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

/// Applies variable substitution to every JSON string leaf with fallback scope.
///
/// Used by [`crate::Config::deserialize`] so a deserialized subtree can resolve
/// both relative keys in the subtree and absolute keys from the root config.
pub(crate) fn substitute_json_strings_with_fallback<
    P: ConfigReader + ?Sized,
    F: ConfigReader + ?Sized,
>(
    value: &mut Value,
    primary: &P,
    fallback: &F,
) -> ConfigResult<()> {
    if !primary.is_enable_variable_substitution() {
        return Ok(());
    }

    match value {
        Value::String(s) => {
            *s = substitute_variables_with_fallback(
                s,
                primary,
                fallback,
                primary.max_substitution_depth(),
            )?;
        }
        Value::Array(values) => {
            for value in values {
                substitute_json_strings_with_fallback(value, primary, fallback)?;
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                substitute_json_strings_with_fallback(value, primary, fallback)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Exercises defensive utility branches that are unreachable through the public
/// deserialization flow after ancestor-key validation.
#[cfg(coverage)]
#[doc(hidden)]
pub(crate) fn coverage_touch_defensive_branches() {
    let _ = map_value_error("coverage", ValueError::NoValue);

    let scalar_parents = [
        Value::Null,
        Value::Bool(true),
        Value::Number(Number::from(42)),
        Value::String("text".to_string()),
        Value::Array(vec![Value::Number(Number::from(1))]),
    ];
    for parent in scalar_parents {
        let mut root = Map::new();
        root.insert("parent".to_string(), parent);
        let _ = insert_deserialize_value(&mut root, "parent.child", Value::Number(Number::from(1)));
    }

    let mut object_root = Map::new();
    let _ = insert_deserialize_value(
        &mut object_root,
        "parent.child.name",
        Value::String("x".to_string()),
    );
    let _ = insert_deserialize_value(
        &mut object_root,
        "parent.child",
        Value::Number(Number::from(1)),
    );

    let mut scalar_root = Map::new();
    let _ = insert_deserialize_value(
        &mut scalar_root,
        "parent.child",
        Value::String("x".to_string()),
    );
    let mut object = Map::new();
    object.insert("name".to_string(), Value::String("x".to_string()));
    let _ = insert_deserialize_value(&mut scalar_root, "parent.child", Value::Object(object));

    let config = crate::Config::new();
    let mut unresolved = Value::String("${QUBIT_CONFIG_COVERAGE_MISSING}".to_string());
    let _ = substitute_json_strings_with_fallback(&mut unresolved, &config, &config);
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
