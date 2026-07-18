// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
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

use std::path::{
    Path,
    PathBuf,
};

use qubit_sanitize::redacted_debug;

use crate::{
    Config,
    ConfigError,
    ConfigResult,
};

use super::{
    ConfigSource,
    config_source::load_transactionally,
};

/// Configuration source that loads from `.env` format files
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{EnvFileConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let temp_dir = tempfile::tempdir().unwrap();
/// let path = temp_dir.path().join(".env");
/// std::fs::write(&path, "PORT=8080\n").unwrap();
/// let source = EnvFileConfigSource::from_file(path);
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// let port = config.get::<String>("PORT").unwrap();
/// assert_eq!(port, "8080");
/// ```
#[derive(Debug, Clone)]
pub struct EnvFileConfigSource {
    path: PathBuf,
}

/// Maps a dotenv parser error without exposing the offending assignment.
///
/// # Parameters
///
/// * `path` - Path of the dotenv file being loaded.
/// * `error` - Parser or I/O error returned by dotenvy.
///
/// # Returns
///
/// A [`ConfigError::IoError`] preserving its I/O kind, or a value-redacted
/// [`ConfigError::ParseError`].
fn map_dotenv_error(path: &Path, error: dotenvy::Error) -> ConfigError {
    match error {
        dotenvy::Error::Io(source) => {
            ConfigError::IoError(std::io::Error::new(
                source.kind(),
                format!(
                    "Failed to read .env file '{}': {source}",
                    path.display(),
                ),
            ))
        }
        dotenvy::Error::LineParse(line, error_index) => {
            ConfigError::ParseError(format!(
                "Failed to parse .env file '{}' at line index \
                 {error_index}: {:?}",
                path.display(),
                redacted_debug(&line),
            ))
        }
        error => ConfigError::ParseError(format!(
            "Failed to parse .env file '{}': {:?}",
            path.display(),
            redacted_debug(&error),
        )),
    }
}

impl EnvFileConfigSource {
    /// Creates a new `EnvFileConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.env` file
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for EnvFileConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        load_transactionally(self, config)
    }

    fn load_into(&self, config: &mut Config) -> ConfigResult<()> {
        let iter = dotenvy::from_path_iter(&self.path)
            .map_err(|error| map_dotenv_error(&self.path, error))?;

        for item in iter {
            let (key, value) =
                item.map_err(|error| map_dotenv_error(&self.path, error))?;
            config.set(&key, value)?;
        }

        Ok(())
    }
}
