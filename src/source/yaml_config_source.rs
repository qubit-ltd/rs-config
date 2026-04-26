/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
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
//! becomes `server.host = "localhost"` and `server.port = "8080"`.
//!
//! Arrays are stored as multi-value properties.
//!
//! # Author
//!
//! Haixing Hu

use std::path::{Path, PathBuf};

use serde_yaml::Value as YamlValue;

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

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
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct YamlConfigSource {
    path: PathBuf,
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
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for YamlConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read YAML file '{}': {}", self.path.display(), e),
            ))
        })?;

        let value: YamlValue = serde_yaml::from_str(&content).map_err(|e| {
            ConfigError::ParseError(format!(
                "Failed to parse YAML file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        flatten_yaml_value("", &value, config)
    }
}

/// Recursively flattens a YAML value into the config using dot-separated keys.
///
/// Scalar types are stored with their native types where possible:
/// - Integer numbers → i64
/// - Floating-point numbers → f64
/// - Booleans → bool
/// - Strings → String
/// - Null → empty property (is_null returns true)
pub(crate) fn flatten_yaml_value(
    prefix: &str,
    value: &YamlValue,
    config: &mut Config,
) -> ConfigResult<()> {
    match value {
        YamlValue::Mapping(map) => {
            for (k, v) in map {
                let key_str = yaml_key_to_string(k)?;
                let key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{}.{}", prefix, key_str)
                };
                flatten_yaml_value(&key, v, config)?;
            }
        }
        YamlValue::Sequence(seq) => {
            flatten_yaml_sequence(prefix, seq, config)?;
        }
        YamlValue::Null => {
            // Null values are stored as empty properties to preserve null semantics.
            use qubit_common::DataType;
            config.set_null(prefix, DataType::String)?;
        }
        YamlValue::Bool(b) => {
            config.set(prefix, *b)?;
        }
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                config.set(prefix, i)?;
            } else {
                let f = n
                    .as_f64()
                    .expect("YAML number should be representable as i64 or f64");
                config.set(prefix, f)?;
            }
        }
        YamlValue::String(s) => {
            config.set(prefix, s.clone())?;
        }
        YamlValue::Tagged(tagged) => {
            flatten_yaml_value(prefix, &tagged.value, config)?;
        }
    }
    Ok(())
}

/// Flattens a YAML sequence into multi-value config entries.
///
/// Homogeneous scalar sequences are stored with their native types.
/// Mixed scalar sequences fall back to string representation.
///
/// Nested structures inside sequences (mapping/sequence/tagged) are rejected
/// with a parse error to avoid silently losing structure information.
fn flatten_yaml_sequence(prefix: &str, seq: &[YamlValue], config: &mut Config) -> ConfigResult<()> {
    if seq.is_empty() {
        return Ok(());
    }

    enum SeqKind {
        Integer,
        Float,
        Bool,
        String,
    }

    let kind = match &seq[0] {
        YamlValue::Number(n) if n.is_i64() => SeqKind::Integer,
        YamlValue::Number(_) => SeqKind::Float,
        YamlValue::Bool(_) => SeqKind::Bool,
        YamlValue::Mapping(_) | YamlValue::Sequence(_) | YamlValue::Tagged(_) => {
            return Err(unsupported_yaml_sequence_element_error(prefix, &seq[0]));
        }
        _ => SeqKind::String,
    };

    let all_same = seq.iter().all(|item| match (&kind, item) {
        (SeqKind::Integer, YamlValue::Number(n)) => n.is_i64(),
        (SeqKind::Float, YamlValue::Number(_)) => true,
        (SeqKind::Bool, YamlValue::Bool(_)) => true,
        (SeqKind::String, YamlValue::String(_)) => true,
        _ => false,
    });

    if !all_same {
        for item in seq {
            config.add(prefix, yaml_scalar_to_string(item, prefix)?)?;
        }
        return Ok(());
    }

    match kind {
        SeqKind::Integer => {
            for item in seq {
                let value = item
                    .as_i64()
                    .expect("YAML integer sequence was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        SeqKind::Float => {
            for item in seq {
                let value = item
                    .as_f64()
                    .expect("YAML float sequence was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        SeqKind::Bool => {
            for item in seq {
                let value = item
                    .as_bool()
                    .expect("YAML bool sequence was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        SeqKind::String => {
            for item in seq {
                let value = yaml_scalar_to_string(item, prefix)
                    .expect("YAML string sequence was validated before insertion");
                config.add(prefix, value)?;
            }
        }
    }

    Ok(())
}

/// Converts a YAML key to a string
fn yaml_key_to_string(value: &YamlValue) -> ConfigResult<String> {
    match value {
        YamlValue::String(s) => Ok(s.clone()),
        YamlValue::Number(n) => Ok(n.to_string()),
        YamlValue::Bool(b) => Ok(b.to_string()),
        YamlValue::Null => Ok("null".to_string()),
        _ => Err(ConfigError::ParseError(format!(
            "Unsupported YAML mapping key type: {value:?}"
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
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => {
            Err(unsupported_yaml_sequence_element_error(key, value))
        }
    }
}

/// Builds a parse error for unsupported nested YAML sequence elements.
fn unsupported_yaml_sequence_element_error(key: &str, value: &YamlValue) -> ConfigError {
    let key = if key.is_empty() { "<root>" } else { key };
    ConfigError::ParseError(format!(
        "Unsupported nested YAML structure at key '{key}': {value:?}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Property;
    use std::path::PathBuf;

    use qubit_value::MultiValues;

    fn config_with_final_property(name: &str) -> Config {
        let mut config = Config::new();
        let mut property = Property::with_value(name, MultiValues::String(vec!["old".to_string()]));
        property.set_final(true);
        config.insert_property(name, property).unwrap();
        config
    }

    fn expect_final_error(result: ConfigResult<()>, name: &str) {
        let err = result.expect_err("writing a final YAML property should fail");
        assert_eq!(
            err.to_string(),
            format!("Property '{name}' is final and cannot be overridden"),
        );
    }

    #[test]
    fn test_from_file_stores_path() {
        let path = PathBuf::from("config.yaml");
        let source = YamlConfigSource::from_file(&path);
        let cloned = source.clone();
        assert_eq!(source.path, path);
        assert_eq!(cloned.path, PathBuf::from("config.yaml"));
    }

    #[test]
    fn test_load_yaml_file_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "server:\n  port: 8080\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        source.load(&mut config).unwrap();

        assert_eq!(config.get::<i64>("server.port").unwrap(), 8080);
    }

    #[test]
    fn test_load_missing_yaml_file_returns_io_error() {
        let source = YamlConfigSource::from_file("missing.yaml");
        let mut config = Config::new();

        source
            .load(&mut config)
            .expect_err("missing YAML file should fail");
    }

    #[test]
    fn test_load_invalid_yaml_file_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.yaml");
        std::fs::write(&path, "key: [unterminated\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        source
            .load(&mut config)
            .expect_err("invalid YAML file should fail");
    }

    #[test]
    fn test_yaml_key_to_string_number() {
        let key = YamlValue::Number(serde_yaml::Number::from(42));
        assert_eq!(yaml_key_to_string(&key).unwrap(), "42");
    }

    #[test]
    fn test_yaml_key_to_string_bool() {
        let key = YamlValue::Bool(true);
        assert_eq!(yaml_key_to_string(&key).unwrap(), "true");
    }

    #[test]
    fn test_yaml_key_to_string_null() {
        let key = YamlValue::Null;
        assert_eq!(yaml_key_to_string(&key).unwrap(), "null");
    }

    #[test]
    fn test_yaml_scalar_to_string_bool() {
        assert_eq!(
            yaml_scalar_to_string(&YamlValue::Bool(false), "k").unwrap(),
            "false"
        );
    }

    #[test]
    fn test_yaml_scalar_to_string_null() {
        assert_eq!(yaml_scalar_to_string(&YamlValue::Null, "k").unwrap(), "");
    }

    #[test]
    fn test_yaml_scalar_to_string_sequence_returns_error() {
        yaml_scalar_to_string(&YamlValue::Sequence(vec![]), "arr")
            .expect_err("nested YAML sequence should fail scalar conversion");
    }

    #[test]
    fn test_yaml_scalar_to_string_mapping_returns_error() {
        yaml_scalar_to_string(&YamlValue::Mapping(serde_yaml::Mapping::new()), "obj")
            .expect_err("nested YAML mapping should fail scalar conversion");
    }

    #[test]
    fn test_flatten_yaml_sequence_mixed_int_null_fallback() {
        // Mixed: int + null → falls back to string
        let seq = vec![
            YamlValue::Number(serde_yaml::Number::from(1i64)),
            YamlValue::Null,
        ];
        let mut config = Config::new();
        flatten_yaml_sequence("mixed", &seq, &mut config).unwrap();
        // Should fall back to string representation
        assert!(config.contains("mixed"));
    }

    #[test]
    fn test_flatten_yaml_sequence_mixed_float_string_fallback() {
        // Mixed: float + string → falls back to string
        let seq = vec![
            YamlValue::Number(serde_yaml::Number::from(1.5f64)),
            YamlValue::String("two".to_string()),
        ];
        let mut config = Config::new();
        flatten_yaml_sequence("mixed", &seq, &mut config).unwrap();
        assert!(config.contains("mixed"));
    }

    #[test]
    fn test_flatten_yaml_sequence_mixed_bool_string_fallback() {
        // Mixed: bool + string → falls back to string
        let seq = vec![YamlValue::Bool(true), YamlValue::String("two".to_string())];
        let mut config = Config::new();
        flatten_yaml_sequence("mixed", &seq, &mut config).unwrap();
        assert!(config.contains("mixed"));
    }

    #[test]
    fn test_flatten_yaml_sequence_mixed_string_int_fallback() {
        // Mixed: string + int → falls back to string
        let seq = vec![
            YamlValue::String("one".to_string()),
            YamlValue::Number(serde_yaml::Number::from(2i64)),
        ];
        let mut config = Config::new();
        flatten_yaml_sequence("mixed", &seq, &mut config).unwrap();
        assert!(config.contains("mixed"));
    }

    #[test]
    fn test_flatten_yaml_sequence_mixed_nested_value_returns_error() {
        let seq = vec![
            YamlValue::Number(serde_yaml::Number::from(1i64)),
            YamlValue::Mapping(serde_yaml::Mapping::new()),
        ];
        let mut config = Config::new();
        flatten_yaml_sequence("mixed", &seq, &mut config)
            .expect_err("mixed YAML sequence with a nested value should fail");
    }

    #[test]
    fn test_flatten_yaml_sequence_nested_mapping_returns_error() {
        let seq = vec![YamlValue::Mapping(serde_yaml::Mapping::new())];
        let mut config = Config::new();
        flatten_yaml_sequence("nested", &seq, &mut config)
            .expect_err("nested YAML mapping should fail sequence flattening");
    }

    #[test]
    fn test_flatten_yaml_sequence_nested_sequence_returns_error() {
        let seq = vec![YamlValue::Sequence(vec![YamlValue::Bool(true)])];
        let mut config = Config::new();
        flatten_yaml_sequence("nested", &seq, &mut config)
            .expect_err("nested YAML sequence should fail sequence flattening");
    }

    #[test]
    fn test_flatten_yaml_value_tagged() {
        use serde_yaml::value::Tag;
        use serde_yaml::value::TaggedValue;
        let tagged = YamlValue::Tagged(Box::new(TaggedValue {
            tag: Tag::new("!!str"),
            value: YamlValue::String("hello".to_string()),
        }));
        let mut config = Config::new();
        flatten_yaml_value("key", &tagged, &mut config).unwrap();
        assert_eq!(config.get_string("key").unwrap(), "hello");
    }

    #[test]
    fn test_flatten_yaml_value_number_no_i64() {
        // A very large float that can't be represented as i64
        let num = serde_yaml::Number::from(f64::MAX);
        let val = YamlValue::Number(num);
        let mut config = Config::new();
        flatten_yaml_value("key", &val, &mut config).unwrap();
        assert!(config.contains("key"));
    }

    #[test]
    fn test_flatten_yaml_scalar_respects_final_property() {
        let cases = [
            YamlValue::Null,
            YamlValue::Bool(true),
            YamlValue::Number(serde_yaml::Number::from(1i64)),
            YamlValue::Number(serde_yaml::Number::from(1.5f64)),
            YamlValue::String("value".to_string()),
            YamlValue::Tagged(Box::new(serde_yaml::value::TaggedValue {
                tag: serde_yaml::value::Tag::new("!!str"),
                value: YamlValue::String("tagged".to_string()),
            })),
        ];

        for value in cases {
            let mut config = config_with_final_property("locked");
            expect_final_error(flatten_yaml_value("locked", &value, &mut config), "locked");
        }
    }

    #[test]
    fn test_flatten_yaml_sequence_respects_final_property() {
        let cases = [
            vec![
                YamlValue::Number(serde_yaml::Number::from(1i64)),
                YamlValue::Number(serde_yaml::Number::from(2i64)),
            ],
            vec![
                YamlValue::Number(serde_yaml::Number::from(1.5f64)),
                YamlValue::Number(serde_yaml::Number::from(2.5f64)),
            ],
            vec![YamlValue::Bool(true), YamlValue::Bool(false)],
            vec![
                YamlValue::String("one".to_string()),
                YamlValue::String("two".to_string()),
            ],
            vec![
                YamlValue::Number(serde_yaml::Number::from(1i64)),
                YamlValue::Null,
            ],
        ];

        for values in cases {
            let mut config = config_with_final_property("locked");
            expect_final_error(
                flatten_yaml_sequence("locked", &values, &mut config),
                "locked",
            );
        }
    }

    #[test]
    fn test_unsupported_yaml_sequence_element_error_uses_root_label() {
        let seq = vec![YamlValue::Mapping(serde_yaml::Mapping::new())];
        let mut config = Config::new();
        let err = flatten_yaml_sequence("", &seq, &mut config)
            .expect_err("nested YAML sequence at root should be rejected");
        assert!(err.to_string().contains("<root>"));
    }
}
