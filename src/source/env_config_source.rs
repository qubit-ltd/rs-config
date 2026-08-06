// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow multiple-public-types
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

use std::{
    collections::HashMap,
    ffi::OsStr,
};

use qubit_redact::{
    EnvRedactor,
    redacted_debug,
};

use crate::{
    Config,
    ConfigError,
    ConfigKey,
    ConfigResult,
    utils,
};

use super::{
    ConfigSource,
    SourceLimits,
    source_budget::SourceBudget,
};

/// Options controlling environment-variable key selection and normalization.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvConfigOptions {
    /// Optional prefix used to select environment variables.
    prefix: Option<String>,
    /// Whether the configured prefix is removed from loaded keys.
    strip_prefix: bool,
    /// Whether underscores are converted to dots.
    underscores_to_dots: bool,
    /// Whether loaded keys are lowercased.
    lowercase_keys: bool,
}

impl EnvConfigOptions {
    /// Creates environment options with no prefix filter or key transformation.
    #[inline]
    pub const fn new() -> Self {
        Self {
            prefix: None,
            strip_prefix: false,
            underscores_to_dots: false,
            lowercase_keys: false,
        }
    }

    /// Restricts loading to variables whose names start with `prefix`.
    #[inline]
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Removes the configured prefix from loaded keys.
    #[inline]
    pub const fn strip_prefix(mut self) -> Self {
        self.strip_prefix = true;
        self
    }

    /// Converts underscores in loaded keys to dots.
    #[inline]
    pub const fn underscores_to_dots(mut self) -> Self {
        self.underscores_to_dots = true;
        self
    }

    /// Lowercases loaded keys.
    #[inline]
    pub const fn lowercase_keys(mut self) -> Self {
        self.lowercase_keys = true;
        self
    }
}

impl Default for EnvConfigOptions {
    /// Creates environment options with no prefix filter or key transformation.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

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
/// let config = source.load().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct EnvConfigSource {
    /// Key selection and normalization options.
    options: EnvConfigOptions,
    /// Resource limits for one environment scan.
    limits: SourceLimits,
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
            options: EnvConfigOptions::default(),
            limits: SourceLimits::default(),
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
            options: EnvConfigOptions::new()
                .prefix(prefix)
                .strip_prefix()
                .underscores_to_dots()
                .lowercase_keys(),
            limits: SourceLimits::default(),
        }
    }

    /// Creates a new `EnvConfigSource` with explicit key options.
    ///
    /// # Parameters
    ///
    /// * `options` - Prefix and key transformation options.
    ///
    /// # Returns
    ///
    /// A configured [`EnvConfigSource`].
    #[inline]
    pub fn with_options(options: EnvConfigOptions) -> Self {
        Self {
            options,
            limits: SourceLimits::default(),
        }
    }

    /// Applies resource limits to this source.
    pub const fn with_limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
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

        if self.options.strip_prefix
            && let Some(prefix) = &self.options.prefix
            && result.starts_with(prefix.as_str())
        {
            result = result[prefix.len()..].to_string();
        }

        if self.options.lowercase_keys {
            result = result.to_lowercase();
        }

        if self.options.underscores_to_dots {
            result = result.replace('_', ".");
        }

        result
    }

    /// Returns whether source key transformations can merge distinct names.
    ///
    /// # Returns
    ///
    /// `true` when the source should reject duplicate normalized keys emitted
    /// by a single load operation.
    #[inline]
    fn can_collapse_distinct_keys(&self) -> bool {
        self.options.strip_prefix
            || self.options.underscores_to_dots
            || self.options.lowercase_keys
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
    /// Always `false` on non-Unix platforms because raw environment bytes are
    /// not available through the standard library.
    #[cfg(not(unix))]
    fn non_unicode_env_key_matches_prefix(_key: &OsStr, _prefix: &str) -> bool {
        false
    }

    /// Converts an OS environment key to UTF-8.
    ///
    /// # Parameters
    ///
    /// * `key` - Environment key returned by [`std::env::vars_os`].
    /// * `value` - Environment value paired with `key`.
    ///
    /// # Returns
    ///
    /// The UTF-8 environment key.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ParseError`] with a fixed redaction marker when
    /// the key is not valid Unicode.
    #[inline]
    fn env_key_to_string(key: &OsStr, value: &OsStr) -> ConfigResult<String> {
        key.to_str().map(str::to_owned).ok_or_else(|| {
            let pair = EnvRedactor::default().redact_os_pair(key, value);
            ConfigError::ParseError(format!(
                "Environment variable key is not valid Unicode: {:?}",
                redacted_debug(&pair),
            ))
        })
    }

    /// Converts an OS environment value to UTF-8.
    ///
    /// # Parameters
    ///
    /// * `key` - Environment variable key used as diagnostic context.
    /// * `value` - Environment value returned by [`std::env::vars_os`].
    ///
    /// # Returns
    ///
    /// The UTF-8 environment value.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ParseError`] containing a log-safe pair produced
    /// by [`EnvRedactor`] when the value is not valid Unicode.
    #[inline]
    fn env_value_to_string(key: &OsStr, value: &OsStr) -> ConfigResult<String> {
        value.to_str().map(str::to_owned).ok_or_else(|| {
            let pair = EnvRedactor::default().redact_os_pair(key, value);
            ConfigError::ParseError(format!(
                "Environment variable value is not valid Unicode: {pair}",
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
    fn load(&self) -> ConfigResult<Config> {
        let mut config = Config::new();
        let mut normalized_keys = HashMap::new();
        let mut budget = SourceBudget::new("process environment", self.limits);

        for (key_os, value_os) in std::env::vars_os() {
            // Filter by prefix if set
            if let Some(prefix) = &self.options.prefix
                && !Self::env_key_matches_prefix(&key_os, prefix)
            {
                continue;
            }

            let key = Self::env_key_to_string(&key_os, &value_os)?;
            let value = Self::env_value_to_string(&key_os, &value_os)?;
            budget
                .consume_input_bytes(key.len().saturating_add(value.len()))?;
            let transformed_key = self.transform_key(&key);
            if self.options.strip_prefix || self.options.underscores_to_dots {
                utils::validate_normalized_config_key(&transformed_key, &key)?;
            }
            if self.can_collapse_distinct_keys()
                && let Some(existing) =
                    normalized_keys.insert(transformed_key.clone(), key.clone())
            {
                return Err(ConfigError::KeyConflict {
                    path: transformed_key,
                    existing: format!("environment variable '{existing}'"),
                    incoming: format!("environment variable '{key}'"),
                });
            }
            let _ = ConfigKey::parse(transformed_key.as_str())?;
            budget.check_depth(transformed_key.split('.').count())?;
            budget.consume_properties(1)?;
            config.set(&transformed_key, value)?;
        }

        Ok(config)
    }
}
