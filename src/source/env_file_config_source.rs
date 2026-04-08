/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # `.env` File Configuration Source
//!
//! Loads configuration from `.env` format files (as used by dotenv tools).
//!
//! # Format
//!
//! The `.env` format supports:
//! - `KEY=VALUE` assignments
//! - `# comment` lines
//! - Quoted values: `KEY="value with spaces"` or `KEY='value'`
//! - Export prefix: `export KEY=VALUE` (the `export` keyword is ignored)
//!
//! # Author
//!
//! Haixing Hu

use std::path::{Path, PathBuf};

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

/// Configuration source that loads from `.env` format files
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::source::{EnvFileConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let source = EnvFileConfigSource::from_file(".env");
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct EnvFileConfigSource {
    path: PathBuf,
}

impl EnvFileConfigSource {
    /// Creates a new `EnvFileConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.env` file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for EnvFileConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let iter = dotenvy::from_path_iter(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read .env file '{}': {}", self.path.display(), e),
            ))
        })?;

        for item in iter {
            let (key, value) = item.map_err(|e| {
                ConfigError::ParseError(format!(
                    "Failed to parse .env file '{}': {}",
                    self.path.display(),
                    e
                ))
            })?;
            config.set(&key, value)?;
        }

        Ok(())
    }
}
