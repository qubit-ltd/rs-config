/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
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
//! becomes `server.host = "localhost"` and `server.port = 8080`.
//!
//! Arrays are stored as multi-value properties.
//!

use std::path::{
    Path,
    PathBuf,
};

use serde_norway as yaml_backend;
use yaml_backend::Value as YamlValue;

use crate::{
    Config,
    ConfigError,
    ConfigResult,
};

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

        let value: YamlValue = yaml_backend::from_str(&content).map_err(|e| {
            ConfigError::ParseError(format!(
                "Failed to parse YAML file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        let mut staged = config.clone();
        flatten_yaml_value("", &value, &mut staged)?;
        *config = staged;
        Ok(())
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
            use qubit_datatype::DataType;
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
/// Homogeneous scalar sequences are stored with their native types. Empty
/// sequences are stored as explicit empty string lists because YAML carries no
/// element type for them. Mixed scalar sequences fall back to string
/// representation.
///
/// Nested structures inside sequences (mapping/sequence/tagged) are rejected
/// with a parse error to avoid silently losing structure information.
fn flatten_yaml_sequence(prefix: &str, seq: &[YamlValue], config: &mut Config) -> ConfigResult<()> {
    if seq.is_empty() {
        config.set(prefix, Vec::<String>::new())?;
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
        let values = seq
            .iter()
            .map(|item| yaml_scalar_to_string(item, prefix))
            .collect::<ConfigResult<Vec<_>>>()?;
        config.set(prefix, values)?;
        return Ok(());
    }

    match kind {
        SeqKind::Integer => {
            let values = seq
                .iter()
                .map(|item| {
                    item.as_i64()
                        .expect("YAML integer sequence was validated before insertion")
                })
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        SeqKind::Float => {
            let values = seq
                .iter()
                .map(|item| {
                    item.as_f64()
                        .expect("YAML float sequence was validated before insertion")
                })
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        SeqKind::Bool => {
            let values = seq
                .iter()
                .map(|item| {
                    item.as_bool()
                        .expect("YAML bool sequence was validated before insertion")
                })
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
        }
        SeqKind::String => {
            let values = seq
                .iter()
                .map(|item| {
                    yaml_scalar_to_string(item, prefix)
                        .expect("YAML string sequence was validated before insertion")
                })
                .collect::<Vec<_>>();
            config.set(prefix, values)?;
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
