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
use std::path::{
    Path,
    PathBuf,
};

use toml::{
    Table as TomlTable,
    Value as TomlValue,
};

use crate::{
    Config,
    ConfigError,
    ConfigResult,
    utils,
};

use super::{
    ConfigSource,
    config_source::load_transactionally,
};

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
/// source.load(&mut config).unwrap();
/// assert_eq!(config.get::<i64>("server.port").unwrap(), 8080);
/// ```
#[derive(Debug, Clone)]
pub struct TomlConfigSource {
    path: PathBuf,
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
fn toml_error_location(
    content: &str,
    error: &toml::de::Error,
) -> Option<(usize, usize)> {
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
/// A [`ConfigError::ParseError`] containing safe path, message, and location
/// context.
fn toml_parse_error(
    path: &Path,
    content: &str,
    error: &toml::de::Error,
) -> ConfigError {
    let message = match toml_error_location(content, error) {
        Some((line, column)) => format!(
            "Failed to parse TOML file '{}' at line {line}, column \
             {column}: {}",
            path.display(),
            error.message(),
        ),
        None => format!(
            "Failed to parse TOML file '{}': {}",
            path.display(),
            error.message(),
        ),
    };
    ConfigError::ParseError(message)
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
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for TomlConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        load_transactionally(self, config)
    }

    fn load_into(&self, config: &mut Config) -> ConfigResult<()> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read TOML file '{}': {}",
                    self.path.display(),
                    e
                ),
            ))
        })?;

        let table: TomlTable = content
            .parse()
            .map_err(|error| toml_parse_error(&self.path, &content, &error))?;

        let mut seen = HashSet::new();
        flatten_toml_value("", &TomlValue::Table(table), config, &mut seen)?;
        Ok(())
    }
}

/// Recursively flattens a TOML value into the config using dot-separated keys.
///
/// Scalar types are stored with their native types (integer → i64, float → f64,
/// bool → bool). Empty arrays become concrete empty collections. String and
/// datetime values are stored as `String`.
pub(crate) fn flatten_toml_value(
    prefix: &str,
    value: &TomlValue,
    config: &mut Config,
    seen: &mut HashSet<String>,
) -> ConfigResult<()> {
    match value {
        TomlValue::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_toml_value(&key, v, config, seen)?;
            }
        }
        TomlValue::Array(arr) => {
            // Detect the element type of the first non-table/non-array item.
            // All elements must be the same scalar type; mixed-type arrays fall
            // back to string representation to avoid silent data loss.
            utils::ensure_unique_flattened_key(seen, prefix)?;
            flatten_toml_array(prefix, arr, config)?;
        }
        TomlValue::String(s) => {
            utils::ensure_unique_flattened_key(seen, prefix)?;
            config.set(prefix, s.clone())?;
        }
        TomlValue::Integer(i) => {
            utils::ensure_unique_flattened_key(seen, prefix)?;
            config.set(prefix, *i)?;
        }
        TomlValue::Float(f) => {
            utils::ensure_unique_flattened_key(seen, prefix)?;
            config.set(prefix, *f)?;
        }
        TomlValue::Boolean(b) => {
            utils::ensure_unique_flattened_key(seen, prefix)?;
            config.set(prefix, *b)?;
        }
        TomlValue::Datetime(dt) => {
            utils::ensure_unique_flattened_key(seen, prefix)?;
            config.set(prefix, dt.to_string())?;
        }
    }
    Ok(())
}

/// Flattens a TOML array into multi-value config entries.
///
/// Homogeneous scalar arrays are stored with their native types. Empty arrays
/// are stored as explicit empty string lists because TOML carries no element
/// type for them. Mixed or nested arrays fall back to string representation.
fn flatten_toml_array(
    prefix: &str,
    arr: &[TomlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    if arr.is_empty() {
        config.set(prefix, Vec::<String>::new())?;
        return Ok(());
    }

    match &arr[0] {
        TomlValue::Integer(_)
            if all_toml_values_match(arr, TomlValue::is_integer) =>
        {
            let values = arr
                .iter()
                .filter_map(TomlValue::as_integer)
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        TomlValue::Float(_)
            if all_toml_values_match(arr, TomlValue::is_float) =>
        {
            let values = arr
                .iter()
                .filter_map(TomlValue::as_float)
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        TomlValue::Boolean(_)
            if all_toml_values_match(arr, TomlValue::is_bool) =>
        {
            let values = arr
                .iter()
                .filter_map(TomlValue::as_bool)
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        TomlValue::String(_) | TomlValue::Datetime(_)
            if arr.iter().all(|value| {
                matches!(value, TomlValue::String(_) | TomlValue::Datetime(_))
            }) =>
        {
            let values = arr
                .iter()
                .map(|value| toml_scalar_to_string(value, prefix))
                .collect::<ConfigResult<Vec<_>>>()?;
            config.set(prefix, values)?;
        }
        TomlValue::Table(_) => {
            return Err(ConfigError::ParseError(format!(
                "Unsupported nested TOML table inside array at key '{prefix}'"
            )));
        }
        TomlValue::Array(_) => {
            return Err(ConfigError::ParseError(format!(
                "Unsupported nested TOML array at key '{prefix}'"
            )));
        }
        _ => {
            let values = arr
                .iter()
                .map(|value| toml_scalar_to_string(value, prefix))
                .collect::<ConfigResult<Vec<_>>>()?;
            config.set(prefix, values)?;
        }
    }

    Ok(())
}

/// Tests whether all TOML values satisfy a scalar type predicate.
fn all_toml_values_match(
    values: &[TomlValue],
    predicate: impl Fn(&TomlValue) -> bool,
) -> bool {
    values.iter().all(predicate)
}

/// Converts a TOML scalar value to a string (used as fallback for mixed arrays)
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
