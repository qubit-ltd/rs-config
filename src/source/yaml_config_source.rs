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

/// Recursively flattens a YAML value into the config using dot-separated keys
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
            for item in seq {
                let str_val = yaml_scalar_to_string(item);
                config.add(prefix, str_val)?;
            }
        }
        YamlValue::Null => {
            // Null values are stored as empty string
            config.set(prefix, String::new())?;
        }
        scalar => {
            let str_val = yaml_scalar_to_string(scalar);
            config.set(prefix, str_val)?;
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

/// Converts a YAML scalar value to a string
fn yaml_scalar_to_string(value: &YamlValue) -> String {
    match value {
        YamlValue::String(s) => s.clone(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Null => String::new(),
        YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => String::new(),
    }
}
