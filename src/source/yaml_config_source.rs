// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # YAML File Configuration Source
//!
//! Loads configuration from YAML format files.
//!
//! # Flattening Strategy
//!
//! Nested YAML mappings are flattened using dot-separated keys.
//! For example:
//!
//! ```yaml
//! server:
//!   host: localhost
//!   port: 8080
//! ```
//!
//! becomes `server.host = "localhost"` and `server.port = 8080`.
//!
//! Arrays are stored as multi-value properties.

use std::collections::HashSet;
use std::path::Path;

use qubit_redact::redacted_debug;
use qubit_value::ValueContainer;
use serde_norway as yaml_backend;
use yaml_backend::Value as YamlValue;

use crate::{
    Config,
    ConfigError,
    ConfigResult,
    utils,
};

use super::{
    ConfigSource,
    SourceLimits,
    config_source::load_transactionally,
    source_budget::SourceBudget,
    source_input::SourceInput,
};

/// Configuration source that loads from YAML format files
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{YamlConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let temp_dir = tempfile::tempdir().unwrap();
/// let path = temp_dir.path().join("config.yaml");
/// std::fs::write(&path, "server:\n  port: 8080\n").unwrap();
/// let source = YamlConfigSource::from_file(path);
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// assert_eq!(config.get::<i64>("server.port").unwrap(), 8080);
/// ```
#[derive(Debug, Clone)]
pub struct YamlConfigSource {
    input: SourceInput,
    limits: SourceLimits,
}

/// Builds a YAML parse error without formatting parser-owned input details.
///
/// # Parameters
///
/// * `path` - Path of the YAML file being parsed.
/// * `error` - YAML parser error used only for its public location.
///
/// # Returns
///
/// A [`ConfigError::ParseError`] containing only file and public location
/// context.
fn yaml_parse_error(label: &str, error: &yaml_backend::Error) -> ConfigError {
    let message = match error.location() {
        Some(location) => format!(
            "Failed to parse YAML file '{}' at line {}, column {}: \
             invalid YAML syntax",
            label,
            location.line(),
            location.column(),
        ),
        None => format!(
            "Failed to parse YAML file '{}': invalid YAML syntax",
            label,
        ),
    };
    ConfigError::ParseError(message)
}

impl YamlConfigSource {
    /// Creates a new `YamlConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the YAML file
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            input: SourceInput::File(path.as_ref().to_path_buf()),
            limits: SourceLimits::default(),
        }
    }

    /// Creates a YAML source backed by in-memory text.
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

impl ConfigSource for YamlConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        load_transactionally(self, config)
    }

    fn load_into(&self, config: &mut Config) -> ConfigResult<()> {
        let label = self.input.label("YAML");
        let content = self.input.read_to_string("YAML", self.limits)?;

        let value: YamlValue = yaml_backend::from_str(&content)
            .map_err(|error| yaml_parse_error(&label, &error))?;

        let mut seen = HashSet::new();
        let mut budget = SourceBudget::new(&label, self.limits);
        flatten_yaml_value("", &value, config, &mut seen, &mut budget, 0)?;
        Ok(())
    }
}

/// Recursively flattens a YAML value into the config using dot-separated keys.
///
/// Scalar types are stored with their native types where possible:
/// - Signed integer numbers → i64
/// - Large non-negative integer numbers → u64
/// - Floating-point numbers → f64
/// - Booleans → bool
/// - Strings → String
/// - Null → unset property (`is_null` returns true)
pub(crate) fn flatten_yaml_value(
    prefix: &str,
    value: &YamlValue,
    config: &mut Config,
    seen: &mut HashSet<String>,
    budget: &mut SourceBudget<'_>,
    depth: usize,
) -> ConfigResult<()> {
    budget.check_depth(depth)?;
    match value {
        YamlValue::Mapping(map) => {
            for (k, v) in map {
                let key_str = yaml_key_to_string(k)?;
                let key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{}.{}", prefix, key_str)
                };
                flatten_yaml_value(
                    &key,
                    v,
                    config,
                    seen,
                    budget,
                    depth.saturating_add(1),
                )?;
            }
        }
        YamlValue::Sequence(seq) => {
            ensure_yaml_property(seen, prefix, budget)?;
            flatten_yaml_sequence(prefix, seq, config)?;
        }
        YamlValue::Null => {
            // Null values are stored as empty properties to preserve null
            // semantics.
            use qubit_datatype::DataType;
            ensure_yaml_property(seen, prefix, budget)?;
            config.set_null(prefix, DataType::String)?;
        }
        YamlValue::Bool(b) => {
            ensure_yaml_property(seen, prefix, budget)?;
            config.set(prefix, *b)?;
        }
        YamlValue::Number(n) => {
            ensure_yaml_property(seen, prefix, budget)?;
            if let Some(i) = n.as_i64() {
                config.set(prefix, i)?;
            } else if let Some(i) = n.as_u64() {
                config.set(prefix, i)?;
            } else {
                let f = n.as_f64().expect(
                    "YAML number should be representable as i64, u64, or f64",
                );
                config.set(prefix, f)?;
            }
        }
        YamlValue::String(s) => {
            ensure_yaml_property(seen, prefix, budget)?;
            config.set(prefix, s.clone())?;
        }
        YamlValue::Tagged(tagged) => {
            flatten_yaml_value(
                prefix,
                &tagged.value,
                config,
                seen,
                budget,
                depth,
            )?;
        }
    }
    Ok(())
}

/// Records one flattened YAML property and enforces the property-count limit.
fn ensure_yaml_property(
    seen: &mut HashSet<String>,
    key: &str,
    budget: &mut SourceBudget<'_>,
) -> ConfigResult<()> {
    utils::ensure_unique_flattened_key(seen, key)?;
    budget.consume_properties(1)
}

/// Flattens a YAML sequence into multi-value config entries.
///
/// Homogeneous scalar sequences are stored with their native types. Empty
/// sequences are stored as explicit empty string lists because YAML carries no
/// element type for them. Mixed scalar sequences fall back to string
/// representation.
///
/// Nested structures inside sequences (mapping/sequence/tagged) are rejected
/// with a parse error to avoid silently losing structure information.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the sequence values.
/// * `seq` - YAML sequence to flatten.
/// * `config` - Configuration mutated with the flattened values.
///
/// # Returns
///
/// `Ok(())` after storing the sequence values.
///
/// # Errors
///
/// Returns an error when the sequence contains nested structures or when the
/// configuration rejects the write, for example because the property is final.
fn flatten_yaml_sequence(
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    if seq.is_empty() {
        config.set(prefix, Vec::<String>::new())?;
        return Ok(());
    }

    match &seq[0] {
        YamlValue::Number(number)
            if number.is_i64()
                && seq
                    .iter()
                    .all(|value| matches!(value, YamlValue::Number(number) if number.is_i64())) =>
        {
            set_yaml_sequence_values(prefix, seq, config, YamlValue::as_i64)
        }
        YamlValue::Number(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::Number(number) if number.is_u64())) =>
        {
            set_yaml_sequence_values(prefix, seq, config, YamlValue::as_u64)
        }
        YamlValue::Number(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::Number(number) if number.is_f64())) =>
        {
            set_yaml_sequence_values(prefix, seq, config, YamlValue::as_f64)
        }
        YamlValue::Bool(_) if seq.iter().all(|value| matches!(value, YamlValue::Bool(_))) => {
            set_yaml_sequence_values(prefix, seq, config, YamlValue::as_bool)
        }
        YamlValue::String(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::String(_))) =>
        {
            set_yaml_string_sequence(prefix, seq, config)
        }
        YamlValue::Mapping(_) | YamlValue::Sequence(_) | YamlValue::Tagged(_) => {
            Err(unsupported_yaml_sequence_element_error(prefix, &seq[0]))
        }
        _ => set_yaml_string_sequence(prefix, seq, config),
    }
}

/// Collects homogeneous YAML scalar values and writes them as one property.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the values.
/// * `seq` - Homogeneous YAML scalar sequence.
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
fn set_yaml_sequence_values<T>(
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
    convert: impl Fn(&YamlValue) -> Option<T>,
) -> ConfigResult<()>
where
    Vec<T>: Into<ValueContainer>,
{
    let values = seq.iter().filter_map(convert).collect::<Vec<_>>();
    config.set(prefix, values)
}

/// Converts YAML scalar values to strings and writes them as one property.
///
/// # Parameters
///
/// * `prefix` - Flattened configuration path receiving the values.
/// * `seq` - YAML scalar sequence to convert.
/// * `config` - Configuration mutated with the converted values.
///
/// # Returns
///
/// `Ok(())` after storing the converted values.
///
/// # Errors
///
/// Returns an error when `seq` contains a nested structure or when the
/// configuration rejects the write.
fn set_yaml_string_sequence(
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    let values = seq
        .iter()
        .map(|value| yaml_scalar_to_string(value, prefix))
        .collect::<ConfigResult<Vec<_>>>()?;
    config.set(prefix, values)
}

/// Converts a YAML key to a string
fn yaml_key_to_string(value: &YamlValue) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        YamlValue::Number(n) => Ok(n.to_string()),
        YamlValue::Bool(b) => Ok(b.to_string()),
        YamlValue::Null => Ok("null".to_string()),
        _ => Err(ConfigError::ParseError(format!(
            "Unsupported YAML mapping key type: {:?}",
            redacted_debug(value),
        ))),
    }
}

/// Converts a YAML scalar value to a string (fallback for mixed-type
/// sequences).
///
/// Nested structures are rejected to avoid silently converting them to empty
/// strings.
fn yaml_scalar_to_string(value: &YamlValue, key: &str) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        YamlValue::Number(n) => Ok(n.to_string()),
        YamlValue::Bool(b) => Ok(b.to_string()),
        YamlValue::Null => Ok(String::new()),
        YamlValue::Sequence(_)
        | YamlValue::Mapping(_)
        | YamlValue::Tagged(_) => {
            Err(unsupported_yaml_sequence_element_error(key, value))
        }
    }
}

/// Builds a parse error for unsupported nested YAML sequence elements.
fn unsupported_yaml_sequence_element_error(
    key: &str,
    value: &YamlValue,
) -> ConfigError {
    let key = if key.is_empty() { "<root>" } else { key };
    ConfigError::ParseError(format!(
        "Unsupported nested YAML structure at key '{key}': {:?}",
        redacted_debug(value),
    ))
}
