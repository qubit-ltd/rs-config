/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
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
//! Without a prefix, all environment variables are loaded as-is.
//!

use std::ffi::{OsStr, OsString};

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

/// Configuration source that loads from system environment variables
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{EnvConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// // Load all env vars
/// let source = EnvConfigSource::new();
///
/// // Load only vars with prefix "APP_", strip prefix and normalize key
/// let source = EnvConfigSource::with_prefix("APP_");
///
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// ```
///
#[derive(Debug, Clone)]
pub struct EnvConfigSource {
    /// Optional prefix filter; only variables with this prefix are loaded
    prefix: Option<String>,
    /// Whether to strip the prefix from the key
    strip_prefix: bool,
    /// Whether to convert underscores to dots in the key
    convert_underscores: bool,
    /// Whether to lowercase the key
    lowercase_keys: bool,
}

impl EnvConfigSource {
    /// Creates a new `EnvConfigSource` that loads all environment variables.
    ///
    /// Keys are loaded as-is (no prefix filtering, no transformation).
    ///
    /// # Returns
    ///
    /// A source that ingests every `std::env::vars()` entry.
    #[inline]
    pub fn new() -> Self {
        Self {
            prefix: None,
            strip_prefix: false,
            convert_underscores: false,
            lowercase_keys: false,
        }
    }

    /// Creates a new `EnvConfigSource` that filters by prefix and normalizes
    /// keys.
    ///
    /// Only variables with the given prefix are loaded. The prefix is stripped,
    /// the key is lowercased, and underscores are converted to dots.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The prefix to filter by (e.g., `"APP_"`)
    ///
    /// # Returns
    ///
    /// A source with prefix filtering and key normalization enabled.
    #[inline]
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            strip_prefix: true,
            convert_underscores: true,
            lowercase_keys: true,
        }
    }

    /// Creates a new `EnvConfigSource` with a custom prefix and explicit
    /// options.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The prefix to filter by
    /// * `strip_prefix` - Whether to strip the prefix from the key
    /// * `convert_underscores` - Whether to convert underscores to dots
    /// * `lowercase_keys` - Whether to lowercase the key
    ///
    /// # Returns
    ///
    /// A configured [`EnvConfigSource`].
    #[inline]
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

    /// Transforms an environment variable key according to the source's
    /// settings.
    ///
    /// # Parameters
    ///
    /// * `key` - Original environment variable name.
    ///
    /// # Returns
    ///
    /// The key after optional prefix strip, lowercasing, and underscore
    /// replacement.
    fn transform_key(&self, key: &str) -> String {
        let mut result = key.to_string();

        if self.strip_prefix
            && let Some(prefix) = &self.prefix
            && result.starts_with(prefix.as_str())
        {
            result = result[prefix.len()..].to_string();
        }

        if self.lowercase_keys {
            result = result.to_lowercase();
        }

        if self.convert_underscores {
            result = result.replace('_', ".");
        }

        result
    }

    /// Checks whether an environment variable key matches a UTF-8 prefix.
    ///
    /// # Parameters
    ///
    /// * `key` - Environment variable key from [`std::env::vars_os`].
    /// * `prefix` - UTF-8 prefix configured on this source.
    ///
    /// # Returns
    ///
    /// `true` if the key starts with `prefix`. On Unix, non-Unicode keys are
    /// compared as bytes so unrelated invalid keys can still be skipped by a
    /// prefixed source.
    fn env_key_matches_prefix(key: &OsStr, prefix: &str) -> bool {
        key.to_str().map_or_else(
            || Self::non_unicode_env_key_matches_prefix(key, prefix),
            |key| key.starts_with(prefix),
        )
    }

    /// Checks a non-Unicode environment key against a UTF-8 prefix.
    ///
    /// # Parameters
    ///
    /// * `key` - Non-Unicode environment variable key.
    /// * `prefix` - UTF-8 prefix configured on this source.
    ///
    /// # Returns
    ///
    /// `true` on Unix when the raw key bytes start with the UTF-8 prefix bytes;
    /// `false` on platforms where raw environment bytes are unavailable.
    #[cfg(unix)]
    fn non_unicode_env_key_matches_prefix(key: &OsStr, prefix: &str) -> bool {
        use std::os::unix::ffi::OsStrExt;

        key.as_bytes().starts_with(prefix.as_bytes())
    }

    /// Checks a non-Unicode environment key against a UTF-8 prefix.
    ///
    /// # Parameters
    ///
    /// * `_key` - Non-Unicode environment variable key.
    /// * `_prefix` - UTF-8 prefix configured on this source.
    ///
    /// # Returns
    ///
    /// Always `false` on non-Unix platforms because raw environment bytes are not
    /// available through the standard library.
    #[cfg(not(unix))]
    fn non_unicode_env_key_matches_prefix(_key: &OsStr, _prefix: &str) -> bool {
        false
    }

    /// Converts an OS environment string to UTF-8.
    ///
    /// # Parameters
    ///
    /// * `value` - Environment key or value returned by [`std::env::vars_os`].
    /// * `label` - Human-readable label included in parse errors.
    ///
    /// # Returns
    ///
    /// `Ok(String)` when `value` is valid UTF-8.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ParseError`] when `value` is not valid Unicode,
    /// preserving a lossy representation in the diagnostic message.
    fn env_os_string_to_string(value: OsString, label: &str) -> ConfigResult<String> {
        value.into_string().map_err(|value| {
            ConfigError::ParseError(format!(
                "{label} is not valid Unicode: {}",
                value.to_string_lossy(),
            ))
        })
    }
}

impl Default for EnvConfigSource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for EnvConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let mut staged = config.clone();
        for (key_os, value_os) in std::env::vars_os() {
            // Filter by prefix if set
            if let Some(prefix) = &self.prefix
                && !Self::env_key_matches_prefix(&key_os, prefix)
            {
                continue;
            }

            let key = Self::env_os_string_to_string(key_os, "Environment variable key")?;
            let value = Self::env_os_string_to_string(
                value_os,
                &format!("Value for environment variable '{key}'"),
            )?;
            let transformed_key = self.transform_key(&key);
            staged.set(&transformed_key, value)?;
        }

        *config = staged;
        Ok(())
    }
}
