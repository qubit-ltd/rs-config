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
/// ```rust,ignore
/// use qubit_config::source::{YamlConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let source = YamlConfigSource::from_file("config.yaml");
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
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
            if !config.contains(prefix) {
                config.set_null(prefix, DataType::String)?;
            }
        }
        YamlValue::Bool(b) => {
            config.set(prefix, *b)?;
        }
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                config.set(prefix, i)?;
            } else if let Some(f) = n.as_f64() {
                config.set(prefix, f)?;
            } else {
                config.set(prefix, n.to_string())?;
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
                if let YamlValue::Number(n) = item {
                    if let Some(i) = n.as_i64() {
                        config.add(prefix, i)?;
                    }
                }
            }
        }
        SeqKind::Float => {
            for item in seq {
                if let YamlValue::Number(n) = item {
                    if let Some(f) = n.as_f64() {
                        config.add(prefix, f)?;
                    }
                }
            }
        }
        SeqKind::Bool => {
            for item in seq {
                if let YamlValue::Bool(b) = item {
                    config.add(prefix, *b)?;
                }
            }
        }
        SeqKind::String => {
            for item in seq {
                config.add(prefix, yaml_scalar_to_string(item, prefix)?)?;
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
        let result = yaml_scalar_to_string(&YamlValue::Sequence(vec![]), "arr");
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_yaml_scalar_to_string_mapping_returns_error() {
        let result = yaml_scalar_to_string(&YamlValue::Mapping(serde_yaml::Mapping::new()), "obj");
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
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
    fn test_flatten_yaml_sequence_nested_mapping_returns_error() {
        let seq = vec![YamlValue::Mapping(serde_yaml::Mapping::new())];
        let mut config = Config::new();
        let result = flatten_yaml_sequence("nested", &seq, &mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_flatten_yaml_sequence_nested_sequence_returns_error() {
        let seq = vec![YamlValue::Sequence(vec![YamlValue::Bool(true)])];
        let mut config = Config::new();
        let result = flatten_yaml_sequence("nested", &seq, &mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
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
}
