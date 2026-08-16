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
//! already materialized YAML value tree is flattened.

use std::collections::HashSet;
use std::path::Path;

use qubit_redact::redacted_debug;
use qubit_value::ValueContainer;
use serde_norway::Error as YamlError;
use serde_norway::Value as YamlValue;
use serde_norway::from_str;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::source_input::SourceInput;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
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

/// Rejects YAML anchors and aliases before the YAML backend builds an owned
/// AST.
///
/// Alias expansion can multiply the amount of owned data produced by a parser.
/// The source format does not need anchors for the flattened configuration
/// model, so rejecting their indicator syntax keeps the configured limits
/// meaningful for untrusted input.
///
/// # Parameters
///
/// * `label` - Stable source label used in the returned parse error.
/// * `content` - Complete YAML document to scan.
///
/// # Errors
///
/// Returns a source parse error when an anchor or alias indicator appears
/// outside quoted, commented, or block-scalar content.
fn reject_yaml_aliases(label: &str, content: &str) -> ConfigResult<()> {
    let mut single_quote = false;
    let mut double_quote = false;
    let mut escaped = false;
    let mut comment = false;
    let mut block_scalar_parent_indent = None;

    for (line_index, raw_line) in content.split_inclusive('\n').enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = line.strip_suffix('\r').unwrap_or(line);

        if let Some(parent_indent) = block_scalar_parent_indent {
            let content_indent = line.bytes().take_while(|byte| *byte == b' ').count();
            let is_blank = line[content_indent..].chars().all(char::is_whitespace);
            let is_block_content = is_blank || content_indent > parent_indent;
            if is_block_content {
                continue;
            }
            block_scalar_parent_indent = None;
        }

        let mut previous = None;
        let mut block_scalar_header = None;
        let mut characters = line.char_indices().peekable();
        while let Some((byte_index, character)) = characters.next() {
            if comment {
                previous = Some(character);
                continue;
            }
            if escaped {
                escaped = false;
                previous = Some(character);
                continue;
            }
            if double_quote && character == '\\' {
                escaped = true;
                previous = Some(character);
                continue;
            }
            if !double_quote && character == '\'' {
                single_quote = !single_quote;
                previous = Some(character);
                continue;
            }
            if !single_quote && character == '"' {
                double_quote = !double_quote;
                previous = Some(character);
                continue;
            }
            if single_quote || double_quote {
                previous = Some(character);
                continue;
            }
            if character == '#' {
                comment = true;
                previous = Some(character);
                continue;
            }
            if matches!(character, '|' | '>') && block_scalar_header.is_none() {
                block_scalar_header = yaml_block_scalar_indent(line, byte_index, previous);
            }
            if matches!(character, '&' | '*') {
                let next_is_anchor_character = characters
                    .peek()
                    .is_some_and(|(_, next)| next.is_ascii_alphanumeric() || *next == '_');
                let at_token_boundary = previous.is_none_or(|previous| {
                    previous.is_whitespace() || matches!(previous, ':' | '[' | '{' | ',' | '-')
                });
                if next_is_anchor_character && at_token_boundary {
                    return Err(ConfigError::source_parse_error(
                        label,
                        format!(
                            "YAML anchors and aliases are not supported at line \
                             {line_number}"
                        ),
                    ));
                }
            }

            previous = Some(character);
        }

        comment = false;
        escaped = false;
        if block_scalar_header.is_some() {
            block_scalar_parent_indent =
                Some(line.bytes().take_while(|byte| *byte == b' ').count());
        }
    }

    Ok(())
}

/// Returns the explicit indentation indicator of a YAML block scalar header.
///
/// A `None` result means that the character is not a block scalar indicator.
/// The inner `Option` carries the optional numeric indentation indicator.
///
/// # Parameters
///
/// * `line` - Current YAML source line.
/// * `byte_index` - Byte offset of the candidate `|` or `>` indicator.
/// * `previous` - Character immediately preceding the candidate indicator.
///
/// # Returns
///
/// `Some` when the candidate is a valid block scalar header, with the optional
/// explicit indentation indicator; otherwise `None`.
fn yaml_block_scalar_indent(
    line: &str,
    byte_index: usize,
    previous: Option<char>,
) -> Option<Option<usize>> {
    let at_token_boundary = previous.is_none_or(|previous| {
        previous.is_whitespace() || matches!(previous, ':' | '[' | '{' | ',' | '-')
    });
    if !at_token_boundary {
        return None;
    }

    let indicator = line[byte_index..].chars().next()?;
    let mut remainder = &line[byte_index + indicator.len_utf8()..];
    let mut explicit_indent = None;
    let mut chomping_seen = false;
    for _ in 0..2 {
        let Some(next) = remainder.chars().next() else {
            break;
        };
        match next {
            '+' | '-' if !chomping_seen => {
                chomping_seen = true;
                remainder = &remainder[next.len_utf8()..];
            }
            '1'..='9' if explicit_indent.is_none() => {
                explicit_indent = Some(next.to_digit(10)? as usize);
                remainder = &remainder[next.len_utf8()..];
            }
            _ => break,
        }
    }

    let remainder = remainder.trim_start();
    if remainder.is_empty() || remainder.starts_with('#') {
        Some(explicit_indent)
    } else {
        None
    }
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
#[must_use]
pub struct YamlConfigSourceBuilder {
    input: SourceInput,
    limits: SourceLimits,
}

#[allow(missing_docs)]
impl YamlConfigSourceBuilder {
    pub fn new() -> Self {
        Self {
            input: SourceInput::Content(String::new()),
            limits: SourceLimits::default(),
        }
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.input = SourceInput::Content(content.into());
        self
    }

    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

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
        reject_yaml_aliases(&label, &content)?;

        let value: YamlValue =
            from_str(&content).map_err(|error| yaml_parse_error(&label, &error))?;

        let mut seen = HashSet::new();
        flatten_yaml_value(&label, "", &value, &mut config, &mut seen, session, 0)?;
        context.replace_layer(config);
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
    budget.consume_nodes(1)?;
    match value {
        YamlValue::Mapping(map) => {
            for (k, v) in map {
                let key_str = yaml_key_to_string(k).map_err(|error| {
                    error.with_source_context(source_id, Some(prefix.to_string()), None)
                })?;
                let key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{}.{}", prefix, key_str)
                };
                flatten_yaml_value(
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
            config.set_null(prefix, DataType::String).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        YamlValue::Bool(b) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, *b).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        YamlValue::Number(n) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            if let Some(i) = n.as_i64() {
                config.set(prefix, i).map_err(|error| {
                    error.with_source_context(source_id, Some(prefix.to_string()), None)
                })?;
            } else if let Some(i) = n.as_u64() {
                config.set(prefix, i).map_err(|error| {
                    error.with_source_context(source_id, Some(prefix.to_string()), None)
                })?;
            } else {
                let f = n
                    .as_f64()
                    .expect("YAML number should be representable as i64, u64, or f64");
                config.set(prefix, f).map_err(|error| {
                    error.with_source_context(source_id, Some(prefix.to_string()), None)
                })?;
            }
        }
        YamlValue::String(s) => {
            ensure_yaml_property(source_id, seen, prefix, budget)?;
            config.set(prefix, s.clone()).map_err(|error| {
                error.with_source_context(source_id, Some(prefix.to_string()), None)
            })?;
        }
        YamlValue::Tagged(tagged) => {
            flatten_yaml_value(
                source_id,
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
fn flatten_yaml_sequence(
    source_id: &str,
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
) -> ConfigResult<()> {
    if seq.is_empty() {
        config.set(prefix, Vec::<String>::new()).map_err(|error| {
            error.with_source_context(source_id, Some(prefix.to_string()), None)
        })?;
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
        YamlValue::String(_)
            if seq
                .iter()
                .all(|value| matches!(value, YamlValue::String(_))) =>
        {
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
fn set_yaml_string_sequence(
    source_id: &str,
    prefix: &str,
    seq: &[YamlValue],
    config: &mut Config,
) -> ConfigResult<()> {
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
        _ => Err(ConfigError::ParseError(
            "YAML mapping keys must be strings".to_string(),
        )),
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
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => {
            Err(ConfigError::ParseError(format!(
                "Unsupported nested YAML structure at key '{key}': {:?}",
                redacted_debug(value),
            )))
        }
    }
}

/// Reports whether two YAML scalar values have the same conversion shape.
fn same_yaml_scalar_kind(first: &YamlValue, other: &YamlValue) -> bool {
    match (first, other) {
        (YamlValue::Null, YamlValue::Null)
        | (YamlValue::Bool(_), YamlValue::Bool(_))
        | (YamlValue::String(_), YamlValue::String(_)) => true,
        (YamlValue::Number(first), YamlValue::Number(other)) => {
            first.is_i64() == other.is_i64()
                && first.is_u64() == other.is_u64()
                && first.is_f64() == other.is_f64()
        }
        _ => false,
    }
}
