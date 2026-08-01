// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Configuration source loading operations.

use std::path::Path;

use super::Config;
use crate::ConfigResult;
#[cfg(feature = "env-file")]
use crate::source::EnvFileConfigSource;
#[cfg(feature = "toml")]
use crate::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use crate::source::YamlConfigSource;
use crate::source::{
    ConfigSource,
    EnvConfigOptions,
    EnvConfigSource,
    PropertiesConfigSource,
};

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
    /// Returns any [`ConfigError`] produced by the source while loading or by
    /// the underlying config mutation methods.
    #[inline]
    pub fn from_source(source: &dyn ConfigSource) -> ConfigResult<Self> {
        let mut config = Self::new();
        source.load(&mut config)?;
        Ok(config)
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
    /// Returns [`ConfigError`] if a matching environment key or value is not
    /// valid Unicode, or if setting a loaded property fails.
    #[inline]
    pub fn from_env() -> ConfigResult<Self> {
        let source = EnvConfigSource::new();
        Self::from_source(&source)
    }

    /// Creates a configuration from environment variables with a prefix.
    ///
    /// Only variables starting with `prefix` are loaded. The prefix is
    /// stripped, the remaining key is lowercased, and underscores are
    /// converted to dots.
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
    /// Returns [`ConfigError`] if a matching environment key or value is not
    /// valid Unicode, or if setting a loaded property fails.
    #[inline]
    pub fn from_env_prefix(prefix: &str) -> ConfigResult<Self> {
        let source = EnvConfigSource::with_prefix(prefix);
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
    /// Returns [`ConfigError`] if a matching environment key or value is not
    /// valid Unicode, or if setting a loaded property fails.
    #[inline]
    pub fn from_env_options(options: EnvConfigOptions) -> ConfigResult<Self> {
        let source = EnvConfigSource::with_options(options);
        Self::from_source(&source)
    }

    /// Creates a configuration from a TOML file.
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
    /// Returns [`ConfigError::IoError`] if the file cannot be read,
    /// [`ConfigError::ParseError`] if the TOML cannot be parsed, or another
    /// [`ConfigError`] if setting a loaded property fails.
    #[cfg(feature = "toml")]
    #[inline]
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = TomlConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a YAML file.
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
    /// Returns [`ConfigError::IoError`] if the file cannot be read,
    /// [`ConfigError::ParseError`] if the YAML cannot be parsed, or another
    /// [`ConfigError`] if setting a loaded property fails.
    #[cfg(feature = "yaml")]
    #[inline]
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = YamlConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a Java `.properties` file.
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
    /// Returns [`ConfigError::IoError`] if the file cannot be read, or another
    /// [`ConfigError`] if setting a loaded property fails.
    #[inline]
    pub fn from_properties_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = PropertiesConfigSource::from_file(path);
        Self::from_source(&source)
    }

    /// Creates a configuration from a `.env` file.
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
    /// Returns [`ConfigError::IoError`] if the file cannot be read,
    /// [`ConfigError::ParseError`] if dotenv parsing fails, or another
    /// [`ConfigError`] if setting a loaded property fails.
    #[cfg(feature = "env-file")]
    #[inline]
    pub fn from_env_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = EnvFileConfigSource::from_file(path);
        Self::from_source(&source)
    }
}
