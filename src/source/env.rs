/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # System Environment Variable Configuration Source
//!
//! Loads configuration from the current process's environment variables.
//!
//! # Key Transformation
//!
//! When a prefix is set, only variables matching the prefix are loaded, and
//! the prefix is stripped from the key name. The key is then lowercased and
//! underscores are converted to dots to produce the config key.
//!
//! For example, with prefix `APP_`:
//! - `APP_SERVER_HOST=localhost` → `server.host = "localhost"`
//! - `APP_SERVER_PORT=8080` → `server.port = "8080"`
//!
//! Without a prefix, all environment variables are loaded as-is (lowercased,
//! underscores converted to dots).
//!
//! # Author
//!
//! Haixing Hu

use crate::{Config, ConfigResult};

use super::ConfigSource;

/// Configuration source that loads from system environment variables
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::source::{EnvSource, ConfigSource};
/// use qubit_config::Config;
///
/// // Load all env vars
/// let source = EnvSource::new();
///
/// // Load only vars with prefix "APP_", strip prefix and normalize key
/// let source = EnvSource::with_prefix("APP_");
///
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct EnvSource {
    /// Optional prefix filter; only variables with this prefix are loaded
    prefix: Option<String>,
    /// Whether to strip the prefix from the key
    strip_prefix: bool,
    /// Whether to convert underscores to dots in the key
    convert_underscores: bool,
    /// Whether to lowercase the key
    lowercase_keys: bool,
}

impl EnvSource {
    /// Creates a new `EnvSource` that loads all environment variables
    ///
    /// Keys are loaded as-is (no prefix filtering, no transformation).
    pub fn new() -> Self {
        Self {
            prefix: None,
            strip_prefix: false,
            convert_underscores: false,
            lowercase_keys: false,
        }
    }

    /// Creates a new `EnvSource` that filters by prefix and normalizes keys
    ///
    /// Only variables with the given prefix are loaded. The prefix is stripped,
    /// the key is lowercased, and underscores are converted to dots.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The prefix to filter by (e.g., `"APP_"`)
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            strip_prefix: true,
            convert_underscores: true,
            lowercase_keys: true,
        }
    }

    /// Creates a new `EnvSource` with a custom prefix and explicit options
    ///
    /// # Parameters
    ///
    /// * `prefix` - The prefix to filter by
    /// * `strip_prefix` - Whether to strip the prefix from the key
    /// * `convert_underscores` - Whether to convert underscores to dots
    /// * `lowercase_keys` - Whether to lowercase the key
    pub fn with_options(
        prefix: &str,
        strip_prefix: bool,
        convert_underscores: bool,
        lowercase_keys: bool,
    ) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            strip_prefix,
            convert_underscores,
            lowercase_keys,
        }
    }

    /// Transforms an environment variable key according to the source's settings
    fn transform_key(&self, key: &str) -> String {
        let mut result = key.to_string();

        if self.strip_prefix {
            if let Some(prefix) = &self.prefix {
                if result.starts_with(prefix.as_str()) {
                    result = result[prefix.len()..].to_string();
                }
            }
        }

        if self.lowercase_keys {
            result = result.to_lowercase();
        }

        if self.convert_underscores {
            result = result.replace('_', ".");
        }

        result
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for EnvSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        for (key, value) in std::env::vars() {
            // Filter by prefix if set
            if let Some(prefix) = &self.prefix {
                if !key.starts_with(prefix.as_str()) {
                    continue;
                }
            }

            let transformed_key = self.transform_key(&key);
            config.set(&transformed_key, value)?;
        }

        Ok(())
    }
}
