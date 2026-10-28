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
//!
//! # TODO: Streaming budget enforcement
//!
//! TODO: replace or wrap the third-party YAML parser with an incremental parser
//! that charges token, string, node, and depth budgets before allocating the
//! complete syntax tree. Current node and depth accounting begins while the
//! already materialized YAML value tree is flattened. Flattening checks both
//! the YAML tree depth and the depth of each resulting dot-separated key.

use std::collections::HashSet;
use std::path::Path;

use qubit_value::ValueContainer;
use serde_norway::Error as YamlError;
use serde_norway::Value as YamlValue;
use serde_norway::from_str;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::yaml_alias_guard::inspect_yaml_tokens;
use super::source_input::SourceInput;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::config_path::ensure_config_key;
use crate::utils;

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
/// let config = source.load().unwrap();
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
/// A source-aware [`ConfigError`] containing only file and public location
/// context.
fn yaml_parse_error(label: &str, error: &YamlError) -> ConfigError {
    let message = match error.location() {
        Some(location) => format!(
            "Failed to parse YAML file '{}' at line {}, column {}: \
             invalid YAML syntax",
            label,
            location.line(),
            location.column(),
        ),
        None => format!("Failed to parse YAML file '{}': invalid YAML syntax", label,),
    };
    ConfigError::source_parse_error(label, message)
}

impl YamlConfigSource {
    /// Creates a new `YamlConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the YAML file
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::builder().file(path).build()
    }

    /// Creates a YAML source backed by in-memory text.
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::builder().content(content).build()
    }

    /// Creates a builder for a YAML source.
    pub fn builder() -> YamlConfigSourceBuilder {
        YamlConfigSourceBuilder::new()
    }
}

/// Builder for [`YamlConfigSource`].
#[must_use = "use the returned value"]
pub struct YamlConfigSourceBuilder {
    input: SourceInput,
    limits: SourceLimits,
}

impl YamlConfigSourceBuilder {
    /// Creates a builder using empty in-memory content and default source
    /// limits.
    ///
    /// The empty content is not parsed until the built source is loaded.
    pub fn new() -> Self {
        Self {
            input: SourceInput::Content(String::new()),
            limits: SourceLimits::default(),
        }
    }

    /// Uses `content` as the source input.
    ///
    /// The owned content replaces any file or content selected earlier. It is
    /// parsed only when the built source is loaded, when format and resource
    /// limit errors are reported through [`ConfigSource`].
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.input = SourceInput::Content(content.into());
        self
    }

    /// Uses `path` as the source input.
    ///
    /// The path replaces any file or content selected earlier. The file is not
    /// opened until the built source is loaded, so I/O, format, and resource
    /// limit errors are reported by [`ConfigSource`].
    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    /// Replaces the resource limits enforced while reading, parsing, and
    /// flattening the source.
    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the source without reading or parsing its selected input.
    ///
    /// Input and limit failures remain deferred until the source is loaded
    /// through [`ConfigSource`].
    pub fn build(self) -> YamlConfigSource {
        YamlConfigSource {
            input: self.input,
            limits: self.limits,
        }
    }
}

impl Default for YamlConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for YamlConfigSource {
    fn source_id(&self) -> String {
        self.input.label("YAML")
    }

    fn limits(&self) -> SourceLimits {
        self.limits
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let mut config = Config::new();
        let session = context.session_mut();
        let label = self.input.label("YAML");
        let content = self.input.read_to_string("YAML", session)?;
        let has_value_tokens = inspect_yaml_tokens(&label, &content)?;

        if content.trim().is_empty() {
            context.replace_layer(config);
            return Ok(());
        }

        let value: YamlValue = from_str(&content).map_err(|error| yaml_parse_error(&label, &error))?;

        if matches!(&value, YamlValue::Null) && !has_value_tokens {
            context.replace_layer(config);
            return Ok(());
        }

        let mut seen = HashSet::new();
        flatten_yaml_value(&label, "", &value, &mut config, &mut seen, session, 0)?;
        context.replace_layer(config);
        Ok(())
    }
}

/// Recursively flattens a YAML value into the config using dot-separated keys.
/// Both YAML tree depth and resulting key depth are subject to source limits.
///
/// Scalar types are stored with their native types where possible:
/// - Signed integer numbers → i64
/// - Large non-negative integer numbers → u64
/// - Floating-point numbers → f64
/// - Booleans → bool
/// - Strings → String
/// - Null → unset property (`is_unset` returns true)
pub(crate) fn flatten_yaml_value(
    source_id: &str,
    prefix: &str,
    value: &YamlValue,
    config: &mut Config,
    seen: &mut HashSet<String>,
    budget: &mut SourceLoadSession<'_>,
    depth: usize,
) -> ConfigResult<()> {
    budget.check_depth(depth)?;
    if !prefix.is_empty() {
        budget.check_depth(prefix.split('.').count())?;
    }
    budget.consume_nodes(1)?;
    match value {
        YamlValue::Mapping(map) => {
            for (k, v) in map {
                let key_str = yaml_key_to_string(k)
                    .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
                let failed_member_path = if prefix.is_empty() {
                    key_str.clone()
                } else {
                    format!("{prefix}.{key_str}")
                };
                ensure_config_key(&key_str)
                    .map_err(|error| error.with_source_context(source_id, Some(failed_member_path.clone()), None))?;
                let key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{}.{}", prefix, key_str)
                };
                ensure_config_key(&key)
                    .map_err(|error| error.with_source_context(source_id, Some(failed_member_path), None))?;
                flatten_yaml_value(source_id, &key, v, config, seen, budget, depth.saturating_add(1))?;
            }
        }
        YamlValue::Sequence(seq) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            budget.consume_nodes(seq.len())?;
            flatten_yaml_sequence(source_id, prefix, seq, config)?;
        }
        YamlValue::Null => {
            // Null values are stored as empty properties to preserve null
            // semantics.
            use qubit_datatype::DataType;
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            config
                .set_null(prefix, DataType::String)
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        YamlValue::Bool(b) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, *b)
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        YamlValue::Number(n) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            if let Some(i) = n.as_i64() {
                config
                    .set(prefix, i)
                    .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
            } else if let Some(i) = n.as_u64() {
                config
                    .set(prefix, i)
                    .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
            } else {
                let f = n
                    .as_f64()
                    .expect("YAML number should be representable as i64, u64, or f64");
                config
                    .set(prefix, f)
                    .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
            }
        }
        YamlValue::String(s) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, s.clone())
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        YamlValue::Tagged(tagged) => {
            flatten_yaml_value(source_id, prefix, &tagged.value, config, seen, budget, depth)?;
        }
    }
    Ok(())
}

/// Records one flattened YAML property and enforces the property-count limit.
fn ensure_yaml_property(
    source_id: &str,
    seen: &mut HashSet<String>,
    key: &str,
    budget: &mut SourceLoadSession<'_>,
) -> ConfigResult<()> {
    utils::ensure_unique_flattened_key(seen, key)
        .map_err(|error| error.with_source_context(source_id, Some(key.to_string()), None))?;
    budget.consume_properties(1)
}

/// Flattens a YAML sequence into multi-value config entries.
///
/// Homogeneous scalar sequences are stored with their native types. Empty
/// sequences are stored as explicit empty string lists because YAML carries no
/// element type for them. Heterogeneous scalar sequences are rejected with
/// source location context rather than being coerced to strings.
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
fn flatten_yaml_sequence(source_id: &str, prefix: &str, seq: &[YamlValue], config: &mut Config) -> ConfigResult<()> {
    if seq.is_empty() {
        config
            .set(prefix, Vec::<String>::new())
            .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        return Ok(());
    }

    if let Some(index) = seq.iter().position(|value| {
        matches!(
            value,
            YamlValue::Mapping(_) | YamlValue::Sequence(_) | YamlValue::Tagged(_)
        )
    }) {
        return Err(ConfigError::source_parse_error_at(
            source_id,
            Some(prefix.to_string()),
            Some(index),
            "nested YAML structures inside sequences are unsupported",
        ));
    }

    match &seq[0] {
        YamlValue::Number(number)
            if number.is_i64()
                && seq
                    .iter()
                    .all(|value| matches!(value, YamlValue::Number(number) if number.is_i64())) =>
        {
            set_yaml_sequence_values(source_id, prefix, seq, config, YamlValue::as_i64)
        }
        YamlValue::Number(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::Number(number) if number.is_u64())) =>
        {
            set_yaml_sequence_values(source_id, prefix, seq, config, YamlValue::as_u64)
        }
        YamlValue::Number(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::Number(number) if number.is_f64())) =>
        {
            set_yaml_sequence_values(source_id, prefix, seq, config, YamlValue::as_f64)
        }
        YamlValue::Bool(_) if seq.iter().all(|value| matches!(value, YamlValue::Bool(_))) => {
            set_yaml_sequence_values(source_id, prefix, seq, config, YamlValue::as_bool)
        }
        YamlValue::String(_) if seq.iter().all(|value| matches!(value, YamlValue::String(_))) => {
            set_yaml_string_sequence(source_id, prefix, seq, config)
        }
        YamlValue::Mapping(_) | YamlValue::Sequence(_) | YamlValue::Tagged(_) => {
            unreachable!("nested YAML structures were rejected above")
        }
        _ => {
            let index = seq
                .iter()
                .position(|value| !same_yaml_scalar_kind(&seq[0], value))
                .unwrap_or(0);
            Err(ConfigError::source_parse_error_at(
                source_id,
                Some(prefix.to_string()),
                Some(index),
                "heterogeneous YAML scalar sequences are unsupported",
            ))
        }
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
    source_id: &str,
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
    convert: impl Fn(&YamlValue) -> Option<T>,
) -> ConfigResult<()>
where
    Vec<T>: Into<ValueContainer>,
{
    let values = seq.iter().filter_map(convert).collect::<Vec<_>>();
    config
        .set(prefix, values)
        .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))
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
fn set_yaml_string_sequence(source_id: &str, prefix: &str, seq: &[YamlValue], config: &mut Config) -> ConfigResult<()> {
    let values = seq
        .iter()
        .map(|value| yaml_scalar_to_string(value, prefix))
        .collect::<ConfigResult<Vec<_>>>()?;
    config
        .set(prefix, values)
        .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))
}

/// Converts a YAML mapping key to a string.
///
/// Only string keys are accepted to avoid silently changing the key schema.
fn yaml_key_to_string(value: &YamlValue) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        _ => Err(ConfigError::ParseError("YAML mapping keys must be strings".to_string())),
    }
}

/// Converts a YAML scalar value to a string for a homogeneous string
/// sequence.
///
/// Nested structures are rejected to avoid silently converting them to empty
/// strings.
fn yaml_scalar_to_string(value: &YamlValue, key: &str) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        YamlValue::Number(n) => Ok(n.to_string()),
        YamlValue::Bool(b) => Ok(b.to_string()),
        YamlValue::Null => Ok(String::new()),
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => Err(ConfigError::ParseError(format!(
            "Unsupported nested YAML structure at key '{key}'",
        ))),
    }
}

/// Reports whether two YAML scalar values have the same conversion shape.
fn same_yaml_scalar_kind(first: &YamlValue, other: &YamlValue) -> bool {
    match (first, other) {
        (YamlValue::Null, YamlValue::Null)
        | (YamlValue::Bool(_), YamlValue::Bool(_))
        | (YamlValue::String(_), YamlValue::String(_)) => true,
        (YamlValue::Number(first), YamlValue::Number(other)) => {
            first.is_i64() == other.is_i64() && first.is_u64() == other.is_u64() && first.is_f64() == other.is_f64()
        }
        _ => false,
    }
}
