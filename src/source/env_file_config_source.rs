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

use std::path::Path;

use qubit_redact::redacted_debug;

use crate::{
    Config,
    ConfigError,
    ConfigKey,
    ConfigResult,
};

use super::{
    ConfigSource,
    SourceLimits,
    config_source::load_transactionally,
    source_budget::SourceBudget,
    source_input::SourceInput,
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
    input: SourceInput,
    limits: SourceLimits,
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
fn map_dotenv_error(label: &str, error: dotenvy::Error) -> ConfigError {
    match error {
        dotenvy::Error::Io(source) => {
            ConfigError::IoError(std::io::Error::new(
                source.kind(),
                format!("Failed to read .env source '{label}': {source}"),
            ))
        }
        dotenvy::Error::LineParse(line, error_index) => {
            ConfigError::ParseError(format!(
                "Failed to parse .env file '{}' at line index \
                 {error_index}: {:?}",
                label,
                redacted_debug(&line),
            ))
        }
        error => ConfigError::ParseError(format!(
            "Failed to parse .env file '{}': {:?}",
            label,
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
            input: SourceInput::File(path.as_ref().to_path_buf()),
            limits: SourceLimits::default(),
        }
    }

    /// Creates a dotenv source backed by in-memory text.
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            input: SourceInput::Content(content.into()),
            limits: SourceLimits::default(),
        }
    }

    /// Applies resource limits to this source.
    pub const fn with_limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }
}

impl ConfigSource for EnvFileConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        load_transactionally(self, config)
    }

    fn load_into(&self, config: &mut Config) -> ConfigResult<()> {
        let label = self.input.label(".env");
        let content = self.input.read_to_string(".env", self.limits)?;
        let iter = dotenvy::from_read_iter(content.as_bytes());

        let mut budget = SourceBudget::new(&label, self.limits);
        for item in iter {
            let (key, value) =
                item.map_err(|error| map_dotenv_error(&label, error))?;
            let _ = ConfigKey::parse(key.as_str())?;
            budget.check_depth(key.split('.').count())?;
            budget.consume_properties(1)?;
            config.set(&key, value)?;
        }

        Ok(())
    }
}
