// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # System Environment Variable Configuration Source
//!
//! Loads configuration from the current process's environment variables.
//!
//! # Key Transformation
//!
//! [`EnvConfigSource::new()`] uses the default options and loads names
//! unchanged. [`EnvConfigSource::from_prefix("APP_")`] filters matching
//! variables, strips the prefix, lowercases names, and replaces double
//! underscores with dots. For example, `APP_SERVER__PORT` becomes `server.port`
//! through that constructor.
//!
//! Explicit options keep filtering and transformation independent. With
//! `EnvConfigOptions::builder().prefix("APP_").build()`, a matching name such
//! as `APP_SERVER__PORT` remains `APP_SERVER__PORT`. Enable `strip_prefix()`,
//! `lowercase_keys()`, and `double_underscores_to_dots()` to select
//! transformations. Lowercasing and double-underscore conversion can also be
//! enabled without a prefix filter.
//!
//! When key transformations make distinct environment names collapse to one
//! configuration key, loading fails with [`ConfigError::KeyConflict`] instead
//! of selecting a winner. The conflicting environment names in that error are
//! reported in lexicographic order, so diagnostics do not depend on the
//! operating system's environment-variable iteration order.

use std::ffi::OsStr;
use std::ffi::OsString;

use super::ConfigSource;
use super::EnvConfigOptions;
use super::EnvConfigSourceBuilder;
use super::SourceLimits;
use super::SourceLoadContext;
use crate::Config;
use crate::ConfigError;
use crate::ConfigKey;
use crate::ConfigResult;
use crate::utils;

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
/// let source = EnvConfigSource::from_prefix("APP_");
///
/// let config = source.load().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct EnvConfigSource {
    /// Key selection and normalization options.
    pub(super) options: EnvConfigOptions,
    /// Resource limits for one environment scan.
    pub(super) limits: SourceLimits,
}

impl EnvConfigSource {
    /// Creates a builder initialized with default environment options.
    #[inline]
    pub fn builder() -> EnvConfigSourceBuilder {
        EnvConfigSourceBuilder::new()
    }

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
    /// the key is lowercased, and double underscores are converted to dots.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The prefix to filter by (e.g., `"APP_"`)
    ///
    /// # Returns
    ///
    /// A source with prefix filtering and key normalization enabled.
    #[inline]
    pub fn from_prefix(prefix: &str) -> Self {
        Self::builder()
            .options(
                EnvConfigOptions::builder()
                    .prefix(prefix)
                    .strip_prefix()
                    .double_underscores_to_dots()
                    .lowercase_keys()
                    .build(),
            )
            .build()
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
    pub fn from_options(options: EnvConfigOptions) -> Self {
        Self::builder().options(options).build()
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
    /// The key after optional prefix strip, lowercasing, and double-underscore
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

        if self.options.double_underscores_to_dots {
            result = result.replace("__", ".");
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
        self.options.strip_prefix || self.options.double_underscores_to_dots || self.options.lowercase_keys
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
    /// Returns a source-aware parse error with a fixed redaction marker when
    /// the key is not valid Unicode.
    #[inline]
    fn env_key_to_string(key: &OsStr, _value: &OsStr) -> ConfigResult<String> {
        key.to_str().map(str::to_owned).ok_or_else(|| {
            ConfigError::source_parse_error(
                "process environment",
                "Environment variable key is not valid Unicode: <redacted>",
            )
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
    /// Returns a source-aware parse error containing the escaped Unicode key,
    /// or a fixed redaction marker if the key is not Unicode, and a fixed
    /// redaction marker when the value is not valid Unicode.
    #[inline]
    fn env_value_to_string(key: &OsStr, value: &OsStr) -> ConfigResult<String> {
        value.to_str().map(str::to_owned).ok_or_else(|| {
            ConfigError::source_parse_error(
                "process environment",
                format!(
                    "Environment variable value is not valid Unicode for {}: <redacted>",
                    key.to_str().unwrap_or("<redacted>").escape_debug(),
                ),
            )
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
    fn source_id(&self) -> String {
        "process environment".to_string()
    }

    fn limits(&self) -> SourceLimits {
        self.limits
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        self.load_environment_entries(std::env::vars_os(), context)
    }
}

impl EnvConfigSource {
    fn load_environment_entries<I>(&self, entries: I, context: &mut SourceLoadContext<'_>) -> ConfigResult<()>
    where
        I: IntoIterator<Item = (OsString, OsString)>,
    {
        let mut accepted_entries = Vec::new();
        let session = context.session_mut();

        for (key_os, value_os) in entries {
            // Filter by prefix if set
            if let Some(prefix) = &self.options.prefix
                && !Self::env_key_matches_prefix(&key_os, prefix)
            {
                continue;
            }

            let key = Self::env_key_to_string(&key_os, &value_os)?;
            let value = Self::env_value_to_string(&key_os, &value_os)?;
            session.consume_input_bytes(key.len().saturating_add(value.len()))?;
            let transformed_key = self.transform_key(&key);
            if self.options.strip_prefix || self.options.double_underscores_to_dots {
                utils::validate_normalized_config_key(&transformed_key, &key).map_err(|error| {
                    error.with_source_context("process environment", Some(transformed_key.clone()), None)
                })?;
            }
            let _ = ConfigKey::parse(transformed_key.as_str()).map_err(|error| {
                error.with_source_context("process environment", Some(transformed_key.clone()), None)
            })?;
            session.check_depth(transformed_key.split('.').count())?;
            session.consume_nodes(1)?;
            session.consume_properties(1)?;
            accepted_entries.push((transformed_key, key, value));
        }

        accepted_entries.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));

        if self.can_collapse_distinct_keys() {
            for pair in accepted_entries.windows(2) {
                let [(left_key, left_name, _), (right_key, right_name, _)] = pair else {
                    unreachable!("windows(2) always contains two entries");
                };
                if left_key == right_key && left_name != right_name {
                    return Err(ConfigError::KeyConflict {
                        source_id: Some("process environment".to_string()),
                        path: left_key.clone(),
                        existing: format!("environment variable '{left_name}'"),
                        incoming: format!("environment variable '{right_name}'"),
                    });
                }
            }
        }

        let mut config = Config::new();
        for (normalized_key, _original_name, value) in accepted_entries {
            config.set(&normalized_key, value)?;
        }

        context.replace_layer(config);
        Ok(())
    }
}

#[cfg(test)]
mod permutation_tests;
