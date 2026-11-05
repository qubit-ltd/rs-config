// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configuration source loading operations.

use std::path::Path;

use super::Config;
use crate::ConfigResult;
use crate::source::ConfigSource;
use crate::source::EnvConfigOptions;
use crate::source::EnvConfigSource;
#[cfg(feature = "env-file")]
use crate::source::EnvFileConfigSource;
use crate::source::PropertiesConfigSource;
#[cfg(feature = "toml")]
use crate::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use crate::source::YamlConfigSource;
use crate::source::load_source;

impl Config {
    // ========================================================================
    // Configuration Source Integration
    // ========================================================================

    /// Creates a new configuration by loading a [`ConfigSource`].
    ///
    /// The returned configuration starts empty and is populated by the given
    /// source. This is a convenience constructor for callers that do not need
    /// to customize the target [`Config`] before loading.
    ///
    /// # Parameters
    ///
    /// * `source` - The configuration source to load from.
    ///
    /// # Returns
    ///
    /// A populated configuration.
    ///
    /// # Errors
    ///
    /// Returns any [`crate::ConfigError`] produced by the source while loading
    /// or by the underlying config mutation methods. Loading uses the standard
    /// context executor and calls `load_into`; an override of
    /// `ConfigSource::load` is not invoked. Move custom loading behavior
    /// into `load_into`.
    #[inline]
    pub fn from_source(source: &dyn ConfigSource) -> ConfigResult<Self> {
        load_source(source)
    }

    /// Creates a configuration from all current process environment variables.
    ///
    /// Environment variable names are loaded as-is. Use
    /// [`Self::from_env_prefix`] when the application uses a dedicated prefix
    /// and wants normalized dot-separated keys.
    ///
    /// # Returns
    ///
    /// A configuration populated from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError`] if a matching environment key or value is
    /// not valid Unicode, or if a loaded key/value exceeds the default source
    /// byte, node, assignment, or depth limits, or if a key is invalid
    /// ([`crate::ConfigError::SourceParseError`]). This convenience method
    /// always uses [`crate::source::SourceLimits::default`]. To customize
    /// limits, build an [`EnvConfigSource`] with [`EnvConfigSource::builder`]
    /// and [`crate::source::SourceLimitsBuilder`], then load it with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[inline]
    pub fn from_env() -> ConfigResult<Self> {
        let source = EnvConfigSource::new();
        Self::from_source(&source)
    }

    /// Creates a configuration from environment variables with a prefix.
    ///
    /// Only variables starting with `prefix` are loaded. The prefix is
    /// stripped, the remaining key is lowercased, and double underscores are
    /// converted to dots while single underscores are preserved. Unicode
    /// names use a case-sensitive string prefix comparison. Non-Unicode names
    /// use native-unit prefix matching on Unix (raw bytes) and Windows
    /// (UTF-16 code units), so matching names are rejected as parse errors
    /// while unrelated names are skipped. Other platforms retain a fallback
    /// that skips non-Unicode names because the standard library does not
    /// expose their native units for prefix matching.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Prefix used to select environment variables.
    ///
    /// # Returns
    ///
    /// A configuration populated from matching environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError`] if a matching environment key or value is
    /// not valid Unicode, or if a loaded key/value exceeds the default source
    /// byte, node, assignment, or depth limits, or if normalized names conflict
    /// ([`crate::ConfigError::KeyConflict`]). Invalid normalized keys are
    /// rejected as source parse errors. This convenience method always uses
    /// [`crate::source::SourceLimits::default`]. To customize limits, build an
    /// [`EnvConfigSource`] with [`EnvConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load it with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[inline]
    pub fn from_env_prefix(prefix: &str) -> ConfigResult<Self> {
        let source = EnvConfigSource::from_prefix(prefix);
        Self::from_source(&source)
    }

    /// Creates a configuration from environment variables with explicit key
    /// selection and transformation options.
    ///
    /// # Parameters
    ///
    /// * `options` - Prefix and key transformation options.
    ///
    /// # Returns
    ///
    /// A configuration populated from matching environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError`] if a matching environment key or value is
    /// not valid Unicode, a transformed key is invalid
    /// ([`crate::ConfigError::SourceParseError`]), loading exceeds a source
    /// limit, or enabled normalization maps distinct names to the same key
    /// ([`crate::ConfigError::KeyConflict`]). This convenience method always
    /// uses [`crate::source::SourceLimits::default`]. To customize limits,
    /// build an [`EnvConfigSource`] with [`EnvConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load it with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[inline]
    pub fn from_env_options(options: EnvConfigOptions) -> ConfigResult<Self> {
        let source = EnvConfigSource::from_options(options);
        Self::from_source(&source)
    }

    /// Creates a configuration from a TOML file.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type that implements [`AsRef<Path>`].
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the TOML file.
    ///
    /// # Returns
    ///
    /// A configuration populated from the TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceIoError`] if the file cannot be
    /// read, [`crate::ConfigError::SourceParseError`] if the TOML cannot be
    /// parsed, or another [`crate::ConfigError`] if setting a loaded
    /// property fails. Returns [`crate::ConfigError::SourceLimitExceeded`] if
    /// loading exceeds an input-byte, node, assignment, semantic-depth,
    /// parser-token, or parser-depth limit.
    ///
    /// This convenience method always uses
    /// [`crate::source::SourceLimits::default`]. To customize limits, configure
    /// the file and limits through [`TomlConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load the source with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[cfg(feature = "toml")]
    #[inline]
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = TomlConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a YAML file.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type that implements [`AsRef<Path>`].
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the YAML file.
    ///
    /// # Returns
    ///
    /// A configuration populated from the YAML file.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceIoError`] if the file cannot be
    /// read, [`crate::ConfigError::SourceParseError`] if the YAML cannot be
    /// parsed, or another [`crate::ConfigError`] if setting a loaded
    /// property fails. Returns [`crate::ConfigError::SourceLimitExceeded`] if
    /// loading exceeds an input-byte, node, assignment, semantic-depth,
    /// parser-token, or parser-depth limit.
    ///
    /// This convenience method always uses
    /// [`crate::source::SourceLimits::default`]. To customize limits, configure
    /// the file and limits through [`YamlConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load the source with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[cfg(feature = "yaml")]
    #[inline]
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = YamlConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a Java `.properties` file.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type that implements [`AsRef<Path>`].
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.properties` file.
    ///
    /// # Returns
    ///
    /// A configuration populated from the `.properties` file.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceIoError`] if the file cannot be
    /// read, or another [`crate::ConfigError`] if setting a loaded property
    /// fails. Returns [`crate::ConfigError::SourceLimitExceeded`] if loading
    /// exceeds an input-byte, node, assignment, or semantic-depth limit.
    ///
    /// This convenience method always uses
    /// [`crate::source::SourceLimits::default`]. To customize limits, configure
    /// the file and limits through [`PropertiesConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load the source with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[inline]
    pub fn from_properties_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = PropertiesConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a `.env` file.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type that implements [`AsRef<Path>`].
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.env` file.
    ///
    /// # Returns
    ///
    /// A configuration populated from the `.env` file.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceIoError`] if the file cannot be
    /// read, [`crate::ConfigError::SourceParseError`] if dotenv parsing
    /// fails, or another [`crate::ConfigError`] if setting a loaded
    /// property fails. Returns [`crate::ConfigError::SourceLimitExceeded`] if
    /// loading exceeds an input-byte, node, assignment, or semantic-depth
    /// limit.
    ///
    /// This convenience method always uses
    /// [`crate::source::SourceLimits::default`]. To customize limits, configure
    /// the file and limits through [`EnvFileConfigSource::builder`] and
    /// [`crate::source::SourceLimitsBuilder`], then load the source with
    /// [`Self::from_source`] or [`Self::merge_properties_from_source`].
    #[cfg(feature = "env-file")]
    #[inline]
    pub fn from_env_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = EnvFileConfigSource::from_file(path);
        Self::from_source(&source)
    }
}
