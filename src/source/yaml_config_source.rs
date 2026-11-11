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
//! Parser-token and collection-depth limits are admitted by the scanner before
//! the backend builds a syntax tree. Node and semantic-depth accounting still
//! occurs while the resulting value tree is flattened.

use std::collections::HashSet;
use std::path::Path;

use qubit_value::ValueContainer;
use serde_norway::Error as YamlError;
use serde_norway::Value as YamlValue;
use serde_norway::from_str;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::text_location::text_location;
use super::internal::yaml_alias_guard::inspect_yaml_tokens;
use super::source_input::SourceInput;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::config_path::ensure_config_key;
use crate::utils;

// Defines YamlConfigSource construction without exposing its stored fields.
#[path = "yaml_config_source_builder.rs"]
mod yaml_config_source_builder;
pub use yaml_config_source_builder::YamlConfigSourceBuilder;

/// Configuration source that loads from YAML format files
///
/// # Resource Limits
///
/// Input bytes, parser tokens, and YAML collection depth are admitted before
/// the backend builds a syntax tree. Node, assignment, and semantic key depth
/// limits are checked during flattening. Parser limits do not guarantee a
/// strict allocation or CPU ceiling.
///
/// See the [source-limit guide](https://github.com/qubit-ltd/rs-config/blob/main/doc/user_guide.md#configure-source-limits).
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
    /// Owns either the file path to read at load time or the in-memory YAML
    /// text retained by this source. In-memory content is borrowed during
    /// parsing; file contents are read into an owned buffer.
    input: SourceInput,
    /// Per-source limits applied during input reading, token admission, and
    /// flattening. Active aggregate parent limits are enforced by the load
    /// context's session alongside these local limits.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

/// Builds a YAML parse error without formatting parser-owned input details.
///
/// # Parameters
///
/// * `label` - Diagnostic source identifier, which may be a custom label or a
///   default file/content identifier rather than a filesystem path.
/// * `error` - YAML parser error used only for its public location.
///
/// # Returns
///
/// A source-aware [`ConfigError`] containing the diagnostic source
/// identifier and an available public location, without parser input details.
fn yaml_parse_error(label: &str, content: &str, error: &YamlError) -> ConfigError {
    let location = error
        .location()
        .and_then(|location| text_location(content, location.index()));
    let message = match location {
        Some(location) => format!(
            "Failed to parse YAML file '{}' at line {}, column {}: \
             invalid YAML syntax",
            label,
            location.line(),
            location.column().unwrap_or(1),
        ),
        None => format!("Failed to parse YAML file '{}': invalid YAML syntax", label,),
    };
    let error = ConfigError::source_parse_error(label, message);
    match location {
        Some(location) => error.with_source_location(location),
        None => error,
    }
}

impl YamlConfigSource {
    /// Creates a new `YamlConfigSource` from a file path
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type accepted by `AsRef<Path>`.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the YAML file
    ///
    /// # Returns
    ///
    /// A YAML source that reads from the given file path.
    #[must_use]
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::builder().file(path).build()
    }

    /// Creates a YAML source backed by in-memory text.
    ///
    /// # Parameters
    ///
    /// * `content` - YAML document text to parse when the source loads.
    ///
    /// # Returns
    ///
    /// A YAML source that reads the supplied in-memory content.
    #[must_use]
    #[inline]
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::builder().content(content).build()
    }

    /// Creates a builder for a YAML source.
    ///
    /// # Returns
    ///
    /// A builder initialized with the YAML source defaults.
    #[inline]
    pub fn builder() -> YamlConfigSourceBuilder {
        YamlConfigSourceBuilder::new()
    }
}

/// Runs the YAML backend only after scanner admission succeeds.
///
/// Scanner admission charges parser tokens and verifies collection depth before
/// the backend is invoked. The returned flag distinguishes an input containing
/// value tokens from an empty/comment-only document, whose backend value may be
/// indistinguishable from an explicit YAML null.
///
/// # Parameters
///
/// * `label` - Source identifier attached to admission and parse errors.
/// * `content` - YAML text scanned before backend parsing.
/// * `session` - Source accounting session charged by the scanner.
/// * `backend` - Parser operation invoked only after successful admission.
///
/// # Returns
///
/// A flag indicating whether value tokens were present and the backend result.
///
/// # Errors
///
/// Returns a scanner admission error or the backend's parse error.
///
/// # Type Parameters
///
/// * `T` - Type converted from the selected configuration property.
fn with_yaml_admission<T>(
    label: &str,
    content: &str,
    session: &mut SourceLoadSession<'_>,
    backend: impl FnOnce() -> ConfigResult<T>,
) -> ConfigResult<(bool, T)> {
    let has_value_tokens = inspect_yaml_tokens(label, content, session)?;
    Ok((has_value_tokens, backend()?))
}

impl ConfigSource for YamlConfigSource {
    /// Returns the stable identifier used to report this source during loading.
    ///
    /// # Returns
    ///
    /// The configured diagnostic label, or a label derived from the input.
    fn source_id(&self) -> String {
        self.label.clone().unwrap_or_else(|| self.input.label("YAML"))
    }

    /// Returns the resource limits applied to one load of this source.
    ///
    /// # Returns
    ///
    /// The limits configured for this source.
    #[inline]
    fn limits(&self) -> SourceLimits {
        self.limits
    }

    /// Loads source data into the executor-provided context.
    ///
    /// # Parameters
    ///
    /// * `context` - Context for charging resource use and writing the source
    ///   layer.
    ///
    /// # Returns
    ///
    /// `Ok(())` after replacing the context layer with the loaded YAML data.
    ///
    /// # Errors
    ///
    /// Returns an input, parser, resource-limit, key-validation, or
    /// configuration-write error.
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let mut config = Config::new();
        let session = context.session_mut();
        let label = self.source_id();
        let content = self.input.read_to_string("YAML", session)?;
        let (has_value_tokens, value) = with_yaml_admission(&label, &content, session, || {
            from_str::<YamlValue>(&content).map_err(|error| yaml_parse_error(&label, &content, &error))
        })?;

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
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to key and write errors.
/// * `prefix` - Current flattened path, empty for the root value.
/// * `value` - YAML value to recursively flatten.
/// * `config` - Configuration receiving the flattened properties.
/// * `seen` - Set used to reject duplicate flattened keys.
/// * `budget` - Session charged for node, property and depth limits.
/// * `depth` - Current YAML tree depth, with the root at zero.
///
/// # Returns
///
/// `Ok(())` after the value and all descendants have been flattened.
///
/// # Errors
///
/// Returns an error when a source limit, key validation, duplicate-key check,
/// unsupported YAML shape, or configuration write fails.
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
            if !seq.is_empty() {
                budget.check_depth(depth.saturating_add(1))?;
            }
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
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to duplicate-key errors.
/// * `seen` - Set of flattened keys already written.
/// * `key` - Flattened property key to record.
/// * `budget` - Session charged for one emitted property.
///
/// # Returns
///
/// `Ok(())` when the key is unique and the property budget accepts it.
///
/// # Errors
///
/// Returns an error for a duplicate key or exceeded property limit.
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
/// Homogeneous scalar sequences are stored with their native types; null
/// sequence elements are unsupported. Empty
/// sequences are stored as explicit empty string lists because YAML carries no
/// element type for them. Heterogeneous scalar sequences are rejected with
/// source location context rather than being coerced to strings.
///
/// Nested structures inside sequences (mapping/sequence/tagged) are rejected
/// with a parse error to avoid silently losing structure information.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to parse errors.
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

    if seq.iter().all(|value| matches!(value, YamlValue::Null)) {
        return Err(ConfigError::source_parse_error_at(
            source_id,
            Some(prefix.to_string()),
            Some(0),
            "null YAML sequence elements are unsupported",
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
            let index = if let YamlValue::Number(first_number) = &seq[0] {
                let mut allowed = [first_number.is_i64(), first_number.is_u64(), first_number.is_f64()];
                seq.iter()
                    .enumerate()
                    .skip(1)
                    .find_map(|(index, value)| {
                        let YamlValue::Number(number) = value else {
                            return Some(index);
                        };
                        let incoming = [number.is_i64(), number.is_u64(), number.is_f64()];
                        for category in 0..3 {
                            allowed[category] &= incoming[category];
                        }
                        (!allowed.iter().any(|accepted| *accepted)).then_some(index)
                    })
                    .unwrap_or(0)
            } else {
                seq.iter()
                    .position(|value| !same_yaml_scalar_kind(&seq[0], value))
                    .unwrap_or(0)
            };
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
/// * `source_id` - Source identifier attached to write errors.
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
/// # Type Parameters
///
/// * `T` - Type converted from the selected configuration property.
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

/// Converts a homogeneous YAML string sequence and writes it as one property.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to write errors.
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
/// Returns an error when `seq` contains a non-string value or when the
/// configuration rejects the write. The caller validates the complete
/// sequence category before conversion; the per-value check remains defensive.
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
///
/// # Parameters
///
/// * `value` - YAML value used as a mapping key.
///
/// # Returns
///
/// The mapping key as an owned string.
///
/// # Errors
///
/// Returns a parse error if the key is not a YAML string.
fn yaml_key_to_string(value: &YamlValue) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        _ => Err(ConfigError::ParseError("YAML mapping keys must be strings".to_string())),
    }
}

/// Converts a YAML string value to text.
///
/// The caller validates the complete sequence category before calling this
/// helper. Other values are rejected defensively.
///
/// # Parameters
///
/// * `value` - YAML string value to convert.
/// * `key` - Flattened path included in an unsupported-value error.
///
/// # Returns
///
/// The string value as an owned string.
///
/// # Errors
///
/// Returns a parse error if the value is not a YAML string.
fn yaml_scalar_to_string(value: &YamlValue, key: &str) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        _ => Err(ConfigError::ParseError(format!(
            "Unsupported nested YAML structure at key '{key}'",
        ))),
    }
}

/// Reports whether two YAML scalar values share the same successful conversion
/// category, matching the common representation accepted for a whole sequence.
///
/// # Parameters
///
/// * `first` - Value that defines the scalar category.
/// * `other` - Value to compare with `first`.
///
/// # Returns
///
/// `true` when both values share a supported scalar category.
#[inline]
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

#[cfg(test)]
mod admission_tests {
    use std::cell::Cell;

    use super::YamlValue;
    use super::same_yaml_scalar_kind;
    use super::with_yaml_admission;
    use super::yaml_parse_error;
    use crate::ConfigError;
    use crate::source::SourceLimitKind;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn test_yaml_scalar_categories_remain_distinct() {
        let number = YamlValue::Number(serde_norway::Number::from(7));
        for value in [
            YamlValue::String("text".to_owned()),
            number,
            YamlValue::Bool(true),
            YamlValue::Null,
        ] {
            assert!(same_yaml_scalar_kind(&value, &value));
        }
        assert!(!same_yaml_scalar_kind(&YamlValue::Null, &YamlValue::Bool(false)));
    }

    #[test]
    fn test_backend_parse_error_fallback_keeps_safe_location() {
        let content = "value: [\"parser-secret\"";
        let error =
            serde_norway::from_str::<YamlValue>(content).expect_err("the backend input is intentionally malformed");
        let error = yaml_parse_error("YAML:<memory>", content, &error);

        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.source_location().is_some());
        assert!(!error.to_string().contains("parser-secret"));
    }

    #[test]
    fn test_backend_parse_error_location_normalizes_unicode_crlf_and_eof() {
        let content = "é: ok\r\nvalue: [";
        let error = serde_norway::from_str::<YamlValue>(content).expect_err("unterminated sequence");
        let location = yaml_parse_error("YAML:<memory>", content, &error).source_location();
        assert_eq!(
            location.map(|location| (location.line(), location.column())),
            Some((2, Some(9)))
        );
    }

    #[test]
    fn test_backend_is_not_called_when_token_admission_fails() {
        let called = Cell::new(false);
        let mut session = SourceLoadSession::new("YAML:<memory>", SourceLimits::builder().max_parser_tokens(0).build());

        let error = with_yaml_admission("YAML:<memory>", "value: yes", &mut session, || {
            called.set(true);
            Ok(())
        })
        .expect_err("scanner admission must fail before invoking the backend");

        assert!(!called.get());
        assert!(matches!(
            error,
            ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserTokenCount,
                ..
            }
        ));
    }

    #[test]
    fn test_backend_is_not_called_when_depth_admission_fails() {
        let called = Cell::new(false);
        let mut session = SourceLoadSession::new(
            "YAML:<memory>",
            SourceLimits::builder().max_parser_nesting_depth(0).build(),
        );

        let error = with_yaml_admission("YAML:<memory>", "outer: {inner: value}", &mut session, || {
            called.set(true);
            Ok(())
        })
        .expect_err("depth admission must fail before invoking the backend");

        assert!(!called.get());
        assert!(matches!(
            error,
            ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserNestingDepth,
                ..
            }
        ));
    }
}
