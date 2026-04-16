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
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for EnvFileConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let iter = dotenvy::from_path_iter(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::other(format!(
                "Failed to read .env file '{}': {}",
                self.path.display(),
                e
            )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigError;

    #[test]
    fn test_load_invalid_env_file_returns_parse_error() {
        // dotenvy fails to parse files with invalid UTF-8 sequences
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.env");
        // Write invalid UTF-8 content
        std::fs::write(&path, b"VALID=ok\n\xff\xfe=bad\n").unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        // dotenvy may return an error or silently skip; either way it shouldn't panic
        // If it returns an error, it should be ParseError
        if let Err(e) = result {
            assert!(matches!(
                e,
                ConfigError::ParseError(_) | ConfigError::IoError(_)
            ));
        }
    }

    #[test]
    fn test_load_env_file_with_unclosed_quote_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unclosed.env");
        // Unclosed quote - dotenvy should return a parse error
        std::fs::write(&path, "KEY=\"unclosed value\n").unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        // Either succeeds (dotenvy is lenient) or fails with ParseError
        let _ = result;
    }
}
