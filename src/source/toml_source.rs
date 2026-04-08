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

use toml::Value as TomlValue;

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

/// Configuration source that loads from TOML format files
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::source::{TomlSource, ConfigSource};
/// use qubit_config::Config;
///
/// let source = TomlSource::from_file("config.toml");
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct TomlSource {
    path: PathBuf,
}

impl TomlSource {
    /// Creates a new `TomlSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for TomlSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read TOML file '{}': {}", self.path.display(), e),
            ))
        })?;

        let value: TomlValue = content.parse().map_err(|e| {
            ConfigError::ParseError(format!(
                "Failed to parse TOML file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        flatten_toml_value("", &value, config)
    }
}

/// Recursively flattens a TOML value into the config using dot-separated keys
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
            // Store arrays as multi-value string properties
            for item in arr {
                let str_val = toml_scalar_to_string(item);
                config.add(prefix, str_val)?;
            }
        }
        scalar => {
            let str_val = toml_scalar_to_string(scalar);
            config.set(prefix, str_val)?;
        }
    }
    Ok(())
}

/// Converts a TOML scalar value to a string
fn toml_scalar_to_string(value: &TomlValue) -> String {
    match value {
        TomlValue::String(s) => s.clone(),
        TomlValue::Integer(i) => i.to_string(),
        TomlValue::Float(f) => f.to_string(),
        TomlValue::Boolean(b) => b.to_string(),
        TomlValue::Datetime(dt) => dt.to_string(),
        TomlValue::Array(_) | TomlValue::Table(_) => String::new(),
    }
}
