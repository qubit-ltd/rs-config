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
//!
//! Parser token and conservative nesting-depth budgets are admitted before the
//! TOML backend builds its value tree. Node and output-depth accounting remains
//! in the existing flattening pass.

use std::collections::HashSet;
use std::path::Path;

use qubit_value::ValueContainer;
use toml::Table as TomlTable;
use toml::Value as TomlValue;
use toml::de::Error as TomlError;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::toml_parser_guard::with_toml_admission;
use super::source_input::SourceInput;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::config_path::ensure_config_key;
use crate::utils;

// Defines TomlConfigSource construction without exposing its stored fields.
#[path = "toml_config_source_builder.rs"]
mod toml_config_source_builder;
pub use toml_config_source_builder::TomlConfigSourceBuilder;

/// Configuration source that loads from TOML format files
///
/// # Resource Limits
///
/// Input bytes, lexer tokens, and conservative parser nesting are admitted
/// before parsing. Node, assignment, and semantic output depth limits are
/// checked while flattening the backend's value tree.
///
/// See the [source-limit guide](https://github.com/qubit-ltd/rs-config/blob/main/doc/user_guide.md#configure-source-limits).
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
    /// File or in-memory content to load.
    input: SourceInput,
    /// Resource limits applied while loading.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
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
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    Some((line, column))
}

/// Builds a TOML parse error without formatting source text.
///
/// # Parameters
///
/// * `label` - Diagnostic source identifier, which may be a custom label or a
///   default file/content identifier rather than a filesystem path.
/// * `content` - TOML source used only to calculate line and column.
/// * `error` - TOML parser error supplying a source-independent message.
///
/// # Returns
///
/// A source-aware [`ConfigError`] containing the diagnostic source
/// identifier, a source-independent message, and an available location.
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
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type accepted by `AsRef<Path>`.
    ///
    /// # Returns
    ///
    /// A TOML source that reads from the given file path.
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::builder().file(path).build()
    }

    /// Creates a TOML source backed by in-memory text.
    ///
    /// # Parameters
    ///
    /// * `content` - TOML document text to parse when the source loads.
    ///
    /// # Returns
    ///
    /// A TOML source that reads the supplied in-memory content.
    #[inline]
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::builder().content(content).build()
    }

    /// Creates a builder for a TOML source.
    ///
    /// # Returns
    ///
    /// A builder initialized with the TOML source defaults.
    #[inline]
    pub fn builder() -> TomlConfigSourceBuilder {
        TomlConfigSourceBuilder::new()
    }
}

impl ConfigSource for TomlConfigSource {
    /// Returns the stable identifier used to report this source during loading.
    fn source_id(&self) -> String {
        self.label.clone().unwrap_or_else(|| self.input.label("TOML"))
    }

    /// Returns the resource limits applied to one load of this source.
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
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let mut config = Config::new();
        let session = context.session_mut();
        let label = self.source_id();
        let content = self.input.read_to_string("TOML", session)?;

        let table: TomlTable = with_toml_admission(&label, &content, session, || {
            content
                .parse()
                .map_err(|error| toml_parse_error(&label, &content, &error))
        })?;

        let mut seen = HashSet::new();
        flatten_toml_value(&label, "", &TomlValue::Table(table), &mut config, &mut seen, session, 0)?;
        context.replace_layer(config);
        Ok(())
    }
}

/// Recursively flattens a TOML value into the config using dot-separated keys.
/// Both TOML tree depth and resulting key depth are subject to source limits.
///
/// Scalar types are stored with their native types (integer → i64, float → f64,
/// bool → bool). Empty arrays become concrete empty collections. Strings and
/// datetime values are stored as `String`; within arrays, they share one string
/// category and may be mixed. Other array categories retain their native types
/// and must be homogeneous.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to key and write errors.
/// * `prefix` - Current flattened path, empty for the root value.
/// * `value` - TOML value to recursively flatten.
/// * `config` - Configuration receiving the flattened properties.
/// * `seen` - Set used to reject duplicate flattened keys.
/// * `budget` - Session used to charge node, property and depth limits.
/// * `depth` - Current TOML tree depth, with the root at zero.
///
/// # Returns
///
/// `Ok(())` after the value and all descendants have been flattened.
///
/// # Errors
///
/// Returns an error when a source limit, key validation, duplicate-key check,
/// or configuration write fails.
pub(crate) fn flatten_toml_value(
    source_id: &str,
    prefix: &str,
    value: &TomlValue,
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
        TomlValue::Table(table) => {
            for (k, v) in table {
                let failed_member_path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                ensure_config_key(k)
                    .map_err(|error| error.with_source_context(source_id, Some(failed_member_path.clone()), None))?;
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                ensure_config_key(&key)
                    .map_err(|error| error.with_source_context(source_id, Some(failed_member_path), None))?;
                flatten_toml_value(source_id, &key, v, config, seen, budget, depth.saturating_add(1))?;
            }
        }
        TomlValue::Array(arr) => {
            // All elements must have the same supported scalar type. Mixed or
            // nested array elements produce a source parse error.
            if !arr.is_empty() {
                budget.check_depth(depth.saturating_add(1))?;
            }
            ensure_toml_property(source_id, seen, prefix, budget)?;
            budget.consume_nodes(arr.len())?;
            flatten_toml_array(source_id, prefix, arr, config)?;
        }
        TomlValue::String(s) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, s.clone())
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        TomlValue::Integer(i) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, *i)
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        TomlValue::Float(f) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, *f)
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        TomlValue::Boolean(b) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, *b)
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
        TomlValue::Datetime(dt) => {
            ensure_toml_property(source_id, seen, prefix, budget)?;
            config
                .set(prefix, dt.to_string())
                .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
        }
    }
    Ok(())
}

/// Records one flattened TOML property and enforces the property-count limit.
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
fn ensure_toml_property(
    source_id: &str,
    seen: &mut HashSet<String>,
    key: &str,
    budget: &mut SourceLoadSession<'_>,
) -> ConfigResult<()> {
    utils::ensure_unique_flattened_key(seen, key)
        .map_err(|error| error.with_source_context(source_id, Some(key.to_string()), None))?;
    budget.consume_properties(1)
}

/// Flattens a TOML array into multi-value config entries.
///
/// Homogeneous scalar arrays preserve their native scalar types. Strings and
/// datetime values share one category and are stored as strings, so they may be
/// mixed in an array. Other scalar categories must be homogeneous. Empty arrays
/// are stored as explicit empty string lists because TOML carries no element
/// type for them. Unsupported mixed or nested arrays are rejected with source
/// location context rather than being coerced to strings or flattened.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to parse errors.
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
fn flatten_toml_array(source_id: &str, prefix: &str, arr: &[TomlValue], config: &mut Config) -> ConfigResult<()> {
    if arr.is_empty() {
        config
            .set(prefix, Vec::<String>::new())
            .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))?;
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

/// Reports whether two TOML values have the same supported scalar category.
/// Strings and datetime values share a category; other scalar categories remain
/// distinct.
///
/// # Parameters
///
/// * `first` - Value that defines the category.
/// * `other` - Value to compare with the category of `first`.
///
/// # Returns
///
/// `true` when both values have the same supported scalar category.
#[inline]
fn same_toml_scalar_kind(first: &TomlValue, other: &TomlValue) -> bool {
    matches!(
        (first, other),
        (
            TomlValue::String(_) | TomlValue::Datetime(_),
            TomlValue::String(_) | TomlValue::Datetime(_),
        ) | (TomlValue::Integer(_), TomlValue::Integer(_))
            | (TomlValue::Float(_), TomlValue::Float(_))
            | (TomlValue::Boolean(_), TomlValue::Boolean(_))
    )
}

/// Collects homogeneous TOML scalar values and writes them as one property.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to write errors.
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
/// # Type Parameters
///
/// * `T` - Type converted from the selected configuration property.
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

/// Converts a homogeneous string/datetime TOML array and writes it as one
/// property.
///
/// # Parameters
///
/// * `source_id` - Source identifier attached to write errors.
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
/// Returns an error when `arr` contains a value outside the string/datetime
/// category or when the configuration rejects the write. The caller validates
/// the complete array category before conversion; the per-value check remains
/// defensive.
fn set_toml_string_array(source_id: &str, prefix: &str, arr: &[TomlValue], config: &mut Config) -> ConfigResult<()> {
    let values = arr
        .iter()
        .map(|value| toml_scalar_to_string(value, prefix))
        .collect::<ConfigResult<Vec<_>>>()?;
    config
        .set(prefix, values)
        .map_err(|error| error.with_source_context(source_id, Some(prefix.to_string()), None))
}

/// Tests whether all TOML values satisfy a scalar type predicate.
///
/// # Parameters
///
/// * `values` - TOML values to inspect.
/// * `predicate` - Test applied to each value.
///
/// # Returns
///
/// `true` if every value satisfies `predicate`.
fn all_toml_values_match(values: &[TomlValue], predicate: impl Fn(&TomlValue) -> bool) -> bool {
    values.iter().all(predicate)
}

/// Converts a TOML string or datetime scalar to text.
///
/// The caller validates the complete array category before calling this helper.
/// Other values are rejected defensively.
///
/// # Parameters
///
/// * `value` - TOML string or datetime value to convert.
/// * `key` - Flattened path included in an unsupported-value error.
///
/// # Returns
///
/// The value represented as a string.
///
/// # Errors
///
/// Returns a parse error when `value` is not a string or datetime.
fn toml_scalar_to_string(value: &TomlValue, key: &str) -> ConfigResult<String> {
    match value {
        TomlValue::String(s) => Ok(s.clone()),
        TomlValue::Datetime(dt) => Ok(dt.to_string()),
        _ => {
            let key = if key.is_empty() { "<root>" } else { key };
            Err(ConfigError::ParseError(format!(
                "Unsupported nested TOML structure at key '{}'",
                key
            )))
        }
    }
}

#[cfg(test)]
mod admission_tests {
    use std::cell::Cell;

    use super::TomlValue;
    use super::toml_error_location;
    use super::toml_parse_error;
    use super::toml_scalar_to_string;
    use super::with_toml_admission;
    use crate::ConfigError;
    use crate::source::SourceLimitKind;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn test_string_conversion_accepts_only_string_datetime_values() {
        let datetime = "1979-05-27T07:32:00Z"
            .parse::<toml::value::Datetime>()
            .expect("the test datetime is valid");
        for (value, expected) in [
            (TomlValue::String("value".to_owned()), "value"),
            (TomlValue::Datetime(datetime), "1979-05-27T07:32:00Z"),
        ] {
            assert_eq!(
                toml_scalar_to_string(&value, "value").expect("supported values convert"),
                expected
            );
        }

        let unsupported = [
            TomlValue::Integer(42),
            TomlValue::Float(1.5),
            TomlValue::Boolean(true),
            TomlValue::Array(Vec::new()),
            TomlValue::Table(toml::Table::new()),
        ];

        for value in unsupported {
            assert!(matches!(
                toml_scalar_to_string(&value, "value"),
                Err(ConfigError::ParseError(message))
                    if message == "Unsupported nested TOML structure at key 'value'"
            ));
        }

        assert!(matches!(
            toml_scalar_to_string(&TomlValue::Integer(42), ""),
            Err(ConfigError::ParseError(message))
                if message == "Unsupported nested TOML structure at key '<root>'"
        ));
    }

    #[test]
    fn test_backend_parse_error_fallback_keeps_safe_location() {
        let content = "name = \"parser-secret\"\nvalue = [";
        let error = content
            .parse::<toml::Table>()
            .expect_err("the backend input is intentionally malformed");

        assert_eq!(toml_error_location(content, &error), Some((2, 10)));
        let error = toml_parse_error("TOML:test", content, &error);
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.to_string().contains("line 2, column 10"));
        assert!(!error.to_string().contains("parser-secret"));
    }

    #[test]
    fn test_backend_is_not_called_when_token_admission_fails() {
        let mut session = SourceLoadSession::new("TOML:test", SourceLimits::builder().max_parser_tokens(2).build());
        let called = Cell::new(false);
        let error = with_toml_admission("TOML:test", "wide = [1, 2, 3]", &mut session, || {
            called.set(true);
            Ok(())
        })
        .expect_err("token admission must fail before backend parsing");

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
        let mut session =
            SourceLoadSession::new("TOML:test", SourceLimits::builder().max_parser_nesting_depth(2).build());
        let called = Cell::new(false);
        let error = with_toml_admission("TOML:test", "values = [[1]]", &mut session, || {
            called.set(true);
            Ok(())
        })
        .expect_err("depth admission must fail before backend parsing");

        assert!(!called.get());
        assert!(matches!(
            error,
            ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserNestingDepth,
                limit: 2,
                observed_at_least: 3,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod mapped_write_errors_tests {
    use std::collections::HashSet;

    use super::TomlValue;
    use super::flatten_toml_value;
    use crate::Config;
    use crate::ConfigError;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn test_final_property_write_errors_keep_key_for_every_value_kind() {
        let datetime = "1979-05-27T07:32:00Z"
            .parse::<toml::value::Datetime>()
            .expect("the test datetime is valid");
        let cases = [
            ("string", TomlValue::String("new".to_owned())),
            ("integer", TomlValue::Integer(42)),
            ("float", TomlValue::Float(1.5)),
            ("boolean", TomlValue::Boolean(true)),
            ("datetime", TomlValue::Datetime(datetime)),
            ("empty_array", TomlValue::Array(Vec::new())),
            ("integer_array", TomlValue::Array(vec![TomlValue::Integer(1)])),
            ("float_array", TomlValue::Array(vec![TomlValue::Float(1.5)])),
            ("boolean_array", TomlValue::Array(vec![TomlValue::Boolean(true)])),
            (
                "string_array",
                TomlValue::Array(vec![TomlValue::String("new".to_owned()), TomlValue::Datetime(datetime)]),
            ),
        ];

        for (key, value) in cases {
            let mut config = Config::new();
            config.set(key, "original").expect("initial value is valid");
            config
                .set_final(key, true)
                .expect("the test property can be made final");
            let mut seen = HashSet::new();
            let mut session = SourceLoadSession::new("TOML:test", SourceLimits::default());

            let error = flatten_toml_value("TOML:test", key, &value, &mut config, &mut seen, &mut session, 0)
                .expect_err("writing a final property must fail");

            assert!(matches!(
                error,
                ConfigError::PropertyIsFinal(ref error_key) if error_key == key
            ));
            assert_eq!(config.get::<String>(key).unwrap(), "original");
        }
    }
}
