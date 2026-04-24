/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
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
//! # Author
//!
//! Haixing Hu

use std::path::{Path, PathBuf};

use toml::{Table as TomlTable, Value as TomlValue};

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

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
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct TomlConfigSource {
    path: PathBuf,
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
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read TOML file '{}': {}", self.path.display(), e),
            ))
        })?;

        let table: TomlTable = content.parse().map_err(|e| {
            ConfigError::ParseError(format!(
                "Failed to parse TOML file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        flatten_toml_value("", &TomlValue::Table(table), config)
    }
}

/// Recursively flattens a TOML value into the config using dot-separated keys.
///
/// Scalar types are stored with their native types (integer → i64, float → f64,
/// bool → bool, null/empty → empty property). String and datetime values are
/// stored as `String`.
pub(crate) fn flatten_toml_value(
    prefix: &str,
    value: &TomlValue,
    config: &mut Config,
) -> ConfigResult<()> {
    match value {
        TomlValue::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_toml_value(&key, v, config)?;
            }
        }
        TomlValue::Array(arr) => {
            // Detect the element type of the first non-table/non-array item.
            // All elements must be the same scalar type; mixed-type arrays fall
            // back to string representation to avoid silent data loss.
            flatten_toml_array(prefix, arr, config)?;
        }
        TomlValue::String(s) => {
            config.set(prefix, s.clone())?;
        }
        TomlValue::Integer(i) => {
            config.set(prefix, *i)?;
        }
        TomlValue::Float(f) => {
            config.set(prefix, *f)?;
        }
        TomlValue::Boolean(b) => {
            config.set(prefix, *b)?;
        }
        TomlValue::Datetime(dt) => {
            config.set(prefix, dt.to_string())?;
        }
    }
    Ok(())
}

/// Flattens a TOML array into multi-value config entries.
///
/// Homogeneous scalar arrays are stored with their native types.
/// Mixed or nested arrays fall back to string representation.
fn flatten_toml_array(prefix: &str, arr: &[TomlValue], config: &mut Config) -> ConfigResult<()> {
    if arr.is_empty() {
        return Ok(());
    }

    // Determine the dominant scalar type from the first element.
    enum ArrayKind {
        Integer,
        Float,
        Bool,
        String,
    }

    let kind = match &arr[0] {
        TomlValue::Integer(_) => ArrayKind::Integer,
        TomlValue::Float(_) => ArrayKind::Float,
        TomlValue::Boolean(_) => ArrayKind::Bool,
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
        _ => ArrayKind::String,
    };

    // Check that all elements match the first element's type; fall back to string if not.
    let all_same = arr.iter().all(|item| {
        matches!(
            (&kind, item),
            (ArrayKind::Integer, TomlValue::Integer(_))
                | (ArrayKind::Float, TomlValue::Float(_))
                | (ArrayKind::Bool, TomlValue::Boolean(_))
                | (
                    ArrayKind::String,
                    TomlValue::String(_) | TomlValue::Datetime(_)
                )
        )
    });

    if !all_same {
        // Mixed types → fall back to string
        for item in arr {
            config.add(prefix, toml_scalar_to_string(item, prefix)?)?;
        }
        return Ok(());
    }

    match kind {
        ArrayKind::Integer => {
            for item in arr {
                let value = item
                    .as_integer()
                    .expect("TOML integer array was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        ArrayKind::Float => {
            for item in arr {
                let value = item
                    .as_float()
                    .expect("TOML float array was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        ArrayKind::Bool => {
            for item in arr {
                let value = item
                    .as_bool()
                    .expect("TOML bool array was validated before insertion");
                config.add(prefix, value)?;
            }
        }
        ArrayKind::String => {
            for item in arr {
                config.add(prefix, toml_scalar_to_string(item, prefix)?)?;
            }
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_scalar_to_string_float() {
        let val = TomlValue::Float(1.5);
        assert_eq!(toml_scalar_to_string(&val, "key").unwrap(), "1.5");
    }

    #[test]
    fn test_toml_scalar_to_string_bool() {
        let val = TomlValue::Boolean(true);
        assert_eq!(toml_scalar_to_string(&val, "key").unwrap(), "true");
    }

    #[test]
    fn test_toml_scalar_to_string_nested_array_empty_key() {
        let val = TomlValue::Array(vec![]);
        let result = toml_scalar_to_string(&val, "");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("<root>"));
    }

    #[test]
    fn test_toml_scalar_to_string_nested_table_with_key() {
        let val = TomlValue::Table(toml::Table::new());
        let result = toml_scalar_to_string(&val, "my.key");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("my.key"));
    }

    #[test]
    fn test_flatten_toml_array_mixed_int_string_fallback() {
        // Build a mixed array manually: first element is Integer, second is String
        // This tests the all_same=false branch
        let arr = vec![TomlValue::Integer(1), TomlValue::String("two".to_string())];
        let mut config = Config::new();
        flatten_toml_array("mixed", &arr, &mut config).unwrap();
        // Should fall back to string representation
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_flatten_toml_array_mixed_float_string_fallback() {
        let arr = vec![TomlValue::Float(1.5), TomlValue::String("two".to_string())];
        let mut config = Config::new();
        flatten_toml_array("mixed", &arr, &mut config).unwrap();
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_flatten_toml_array_mixed_bool_string_fallback() {
        let arr = vec![
            TomlValue::Boolean(true),
            TomlValue::String("two".to_string()),
        ];
        let mut config = Config::new();
        flatten_toml_array("mixed", &arr, &mut config).unwrap();
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 2);
    }
}
