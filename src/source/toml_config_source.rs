// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # TOML File Configuration Source
//!
//! Loads configuration from TOML format files.
//!
//! # Flattening Strategy
//!
//! Nested TOML tables are flattened using dot-separated keys.
//! For example:
//!
//! ```toml
//! [server]
//! host = "localhost"
//! port = 8080
//! ```
//!
//! becomes `server.host = "localhost"` and `server.port = 8080`.
//!
//! Arrays are stored as multi-value properties.

use std::collections::HashSet;
use std::path::Path;

use qubit_value::ValueContainer;
use toml::Table as TomlTable;
use toml::Value as TomlValue;
use toml::de::Error as TomlError;

use super::ConfigSource;
use super::SourceLimits;
use super::source_budget::SourceBudget;
use super::source_input::SourceInput;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::utils;

/// Configuration source that loads from TOML format files
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{TomlConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let temp_dir = tempfile::tempdir().unwrap();
/// let path = temp_dir.path().join("config.toml");
/// std::fs::write(&path, "server.port = 8080\n").unwrap();
/// let source = TomlConfigSource::from_file(path);
/// let mut config = Config::new();
/// let config = source.load().unwrap();
/// assert_eq!(config.get::<i64>("server.port").unwrap(), 8080);
/// ```
#[derive(Debug, Clone)]
pub struct TomlConfigSource {
    input: SourceInput,
    limits: SourceLimits,
}

/// Computes a one-based TOML error location from a parser span.
///
/// # Parameters
///
/// * `content` - TOML source used only to calculate line and column.
/// * `error` - TOML parser error providing the byte span.
///
/// # Returns
///
/// `Some((line, column))` when the span starts at a valid UTF-8 boundary,
/// otherwise `None`.
fn toml_error_location(content: &str, error: &TomlError) -> Option<(usize, usize)> {
    let offset = error.span()?.start;
    let prefix = content.get(..offset)?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .map_or(1, |line| line.chars().count() + 1);
    Some((line, column))
}

/// Builds a TOML parse error without formatting source text.
///
/// # Parameters
///
/// * `path` - Path of the TOML file being parsed.
/// * `content` - TOML source used only to calculate line and column.
/// * `error` - TOML parser error supplying a source-independent message.
///
/// # Returns
///
/// A source-aware [`ConfigError`] containing safe path, message, and location
/// context.
fn toml_parse_error(label: &str, content: &str, error: &TomlError) -> ConfigError {
    let message = match toml_error_location(content, error) {
        Some((line, column)) => format!(
            "Failed to parse TOML file '{}' at line {line}, column \
             {column}: {}",
            label,
            error.message(),
        ),
        None => format!("Failed to parse TOML file '{}': {}", label, error.message(),),
    };
    ConfigError::source_parse_error(label, message)
}

impl TomlConfigSource {
    /// Creates a new `TomlConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the TOML file
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            input: SourceInput::File(path.as_ref().to_path_buf()),
            limits: SourceLimits::default(),
        }
    }

    /// Creates a TOML source backed by in-memory text.
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            input: SourceInput::Content(content.into()),
            limits: SourceLimits::default(),
        }
    }

    /// Applies resource limits to this source.
    pub const fn with_limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }
}

impl ConfigSource for TomlConfigSource {
    fn load(&self) -> ConfigResult<Config> {
        let mut config = Config::new();
        let label = self.input.label("TOML");
        let content = self.input.read_to_string("TOML", self.limits)?;

        let table: TomlTable = content
            .parse()
            .map_err(|error| toml_parse_error(&label, &content, &error))?;

        let mut seen = HashSet::new();
        let mut budget = SourceBudget::new(&label, self.limits);
        flatten_toml_value(
            &label,
            "",
            &TomlValue::Table(table),
            &mut config,
            &mut seen,
            &mut budget,
            0,
        )?;
        Ok(config)
    }
}

/// Recursively flattens a TOML value into the config using dot-separated keys.
///
/// Scalar types are stored with their native types (integer → i64, float → f64,
/// bool → bool). Empty arrays become concrete empty collections. String and
/// datetime values are stored as `String`.
pub(crate) fn flatten_toml_value(
    source_id: &str,
    prefix: &str,
    value: &TomlValue,
    config: &mut Config,
    seen: &mut HashSet<String>,
    budget: &mut SourceBudget<'_>,
    depth: usize,
) -> ConfigResult<()> {
    budget.check_depth(depth)?;
    match value {
        TomlValue::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_toml_value(
                    source_id,
                    &key,
                    v,
                    config,
                    seen,
                    budget,
                    depth.saturating_add(1),
                )?;
            }
        }
        TomlValue::Array(arr) => {
            // Detect the element type of the first non-table/non-array item.
            // All elements must be the same scalar type; mixed-type arrays fall
            // back to string representation to avoid silent data loss.
            ensure_toml_property(source_id, seen, prefix, budget)?;
            flatten_toml_array(source_id, prefix, arr, config)?;
        }
        TomlValue::String(s) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, s.clone()).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        TomlValue::Integer(i) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, *i).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        TomlValue::Float(f) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, *f).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        TomlValue::Boolean(b) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, *b).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        TomlValue::Datetime(dt) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, dt.to_string()).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
    }
    Ok(())
}

/// Records one flattened TOML property and enforces the property-count limit.
fn ensure_toml_property(
    source_id: &str,
    seen: &mut HashSet<String>,
    key: &str,
    budget: &mut SourceBudget<'_>,
) -> ConfigResult<()> {
    utils::ensure_unique_flattened_key(seen, key)
        .map_err(|error| error.with_source_context(source_id, Some(key.to_string()), None))?;
    budget.consume_properties(1)
}

/// Flattens a TOML array into multi-value config entries.
///
/// Homogeneous scalar arrays are stored with their native types. Empty arrays
/// are stored as explicit empty string lists because TOML carries no element
/// type for them. Heterogeneous scalar arrays are rejected with source
/// location context rather than being coerced to strings.
/// Nested arrays and tables are rejected because flattening them would lose
/// structure information.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the array values.
/// * `arr` - TOML array to flatten.
/// * `config` - Configuration mutated with the flattened values.
///
/// # Returns
///
/// `Ok(())` after storing the array values.
///
/// # Errors
///
/// Returns an error when the array contains nested structures or when the
/// configuration rejects the write, for example because the property is final.
fn flatten_toml_array(
    source_id: &str,
    prefix: &str,
    arr: &[TomlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    if arr.is_empty() {
        config.set(prefix, Vec::<String>::new()).map_err(|error| {
            error.with_source_context(source_id, Some(prefix.to_string()), None)
        })?;
        return Ok(());
    }

    match &arr[0] {
        TomlValue::Integer(_) if all_toml_values_match(arr, TomlValue::is_integer) => {
            set_toml_array_values(source_id, prefix, arr, config, TomlValue::as_integer)
        }
        TomlValue::Float(_) if all_toml_values_match(arr, TomlValue::is_float) => {
            set_toml_array_values(source_id, prefix, arr, config, TomlValue::as_float)
        }
        TomlValue::Boolean(_) if all_toml_values_match(arr, TomlValue::is_bool) => {
            set_toml_array_values(source_id, prefix, arr, config, TomlValue::as_bool)
        }
        TomlValue::String(_) | TomlValue::Datetime(_)
            if arr
                .iter()
                .all(|value| matches!(value, TomlValue::String(_) | TomlValue::Datetime(_))) =>
        {
            set_toml_string_array(source_id, prefix, arr, config)
        }
        TomlValue::Table(_) | TomlValue::Array(_) => {
            let index = arr
                .iter()
                .position(|value| matches!(value, TomlValue::Table(_) | TomlValue::Array(_)))
                .unwrap_or(0);
            Err(ConfigError::source_parse_error_at(
                source_id,
                Some(prefix.to_string()),
                Some(index),
                "nested TOML structures inside arrays are unsupported",
            ))
        }
        _ => {
            let index = arr
                .iter()
                .position(|value| !same_toml_scalar_kind(&arr[0], value))
                .unwrap_or(0);
            Err(ConfigError::source_parse_error_at(
                source_id,
                Some(prefix.to_string()),
                Some(index),
                "unsupported TOML array element types",
            ))
        }
    }
}

/// Reports whether two TOML values have the same supported scalar shape.
fn same_toml_scalar_kind(first: &TomlValue, other: &TomlValue) -> bool {
    matches!(
        (first, other),
        (TomlValue::String(_), TomlValue::String(_))
            | (TomlValue::Datetime(_), TomlValue::Datetime(_))
            | (TomlValue::Integer(_), TomlValue::Integer(_))
            | (TomlValue::Float(_), TomlValue::Float(_))
            | (TomlValue::Boolean(_), TomlValue::Boolean(_))
    )
}

/// Collects homogeneous TOML scalar values and writes them as one property.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the values.
/// * `arr` - Homogeneous TOML scalar array.
/// * `config` - Configuration mutated with the collected values.
/// * `convert` - Conversion used for each scalar value.
///
/// # Returns
///
/// `Ok(())` after storing the collected values.
///
/// # Errors
///
/// Returns an error when the configuration rejects the write.
fn set_toml_array_values<T>(
    source_id: &str,
    prefix: &str,
    arr: &[TomlValue],
    config: &mut Config,
    convert: impl Fn(&TomlValue) -> Option<T>,
) -> ConfigResult<()>
where
    Vec<T>: Into<ValueContainer>,
{
    let values = arr.iter().filter_map(convert).collect::<Vec<_>>();
    config
        .set(prefix, values)
        .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))
}

/// Converts TOML scalar values to strings and writes them as one property.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the values.
/// * `arr` - TOML scalar array to convert.
/// * `config` - Configuration mutated with the converted values.
///
/// # Returns
///
/// `Ok(())` after storing the converted values.
///
/// # Errors
///
/// Returns an error when `arr` contains a nested structure or when the
/// configuration rejects the write.
fn set_toml_string_array(
    source_id: &str,
    prefix: &str,
    arr: &[TomlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    let values = arr
        .iter()
        .map(|value| toml_scalar_to_string(value, prefix))
        .collect::<ConfigResult<Vec<_>>>()?;
    config
        .set(prefix, values)
        .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))
}

/// Tests whether all TOML values satisfy a scalar type predicate.
fn all_toml_values_match(values: &[TomlValue], predicate: impl Fn(&TomlValue) -> bool) -> bool {
    values.iter().all(predicate)
}

/// Converts a homogeneous TOML string or datetime scalar to text.
fn toml_scalar_to_string(value: &TomlValue, key: &str) -> ConfigResult<String> {
    match value {
        TomlValue::String(s) => Ok(s.clone()),
        TomlValue::Integer(i) => Ok(i.to_string()),
        TomlValue::Float(f) => Ok(f.to_string()),
        TomlValue::Boolean(b) => Ok(b.to_string()),
        TomlValue::Datetime(dt) => Ok(dt.to_string()),
        TomlValue::Array(_) | TomlValue::Table(_) => {
            let key = if key.is_empty() { "<root>" } else { key };
            Err(ConfigError::ParseError(format!(
                "Unsupported nested TOML structure at key '{}'",
                key
            )))
        }
    }
}
