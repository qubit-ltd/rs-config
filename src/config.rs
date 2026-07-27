// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Configuration Manager
//!
//! Provides storage, retrieval, and management of configurations.

#![allow(private_bounds)]

mod internal;

use serde::de::{
    DeserializeOwned,
    Error as _,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};
use std::collections::HashMap;
use std::path::Path;

use self::internal::{
    ConfigSerdeRepr,
    ConfigWire,
    ConfigWireV1,
    ConfigWireV1Ref,
};
use crate::ConfigPropertyMut;
use crate::config_path::{
    ensure_config_key,
    ensure_config_path,
};
use crate::config_reader::ConfigReader;
use crate::config_section::ConfigSection;
use crate::config_serde_ext::ConfigSerdeExt;
use crate::field::ConfigField;
use crate::from::{
    FromConfig,
    IntoConfigDefault,
};
use crate::options::ReadOptions;
#[cfg(feature = "env-file")]
use crate::source::EnvFileConfigSource;
#[cfg(feature = "toml")]
use crate::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use crate::source::YamlConfigSource;
use crate::source::{
    ConfigSource,
    EnvConfigSource,
    PropertiesConfigSource,
};
use crate::utils;
use crate::{
    ConfigError,
    ConfigName,
    ConfigNames,
    ConfigResult,
    Property,
};
use qubit_datatype::{
    DataConversionTarget,
    DataType,
};
use qubit_value::{
    StrictValueListRead,
    StrictValueRead,
    Value as QubitValue,
    ValueContainer,
};

/// Converts deserialized numeric text with the configured conversion policy.
///
/// # Type Parameters
///
/// * `T` - Target numeric type.
///
/// # Parameters
///
/// * `key` - Configuration path used for error context.
/// * `options` - Read options controlling conversion.
/// * `value` - Numeric text to convert.
///
/// # Returns
///
/// The converted number.
///
/// # Errors
///
/// Returns a keyed conversion error when `value` cannot be converted to `T`.
pub(crate) fn convert_deserialize_number<T>(
    key: &str,
    options: &ReadOptions,
    value: String,
) -> ConfigResult<T>
where
    T: DataConversionTarget,
{
    match QubitValue::String(value).to_with::<T>(options.conversion_options()) {
        Ok(value) => Ok(value),
        Err(error) => Err(ConfigError::from((key, error))),
    }
}

/// Configuration Manager
///
/// Manages a set of configuration properties with type-safe read/write
/// interfaces.
///
/// # Features
///
/// - Supports multiple data types
/// - Supports variable substitution (`${var_name}` format)
/// - Supports configuration merging
/// - Supports final value protection
/// - Thread-safe (when wrapped in `Arc<RwLock<Config>>`)
///
/// # Persistence
///
/// Serde serialization emits the stable V1 JSON persistence format with an
/// explicit `version` field and lexically ordered property keys.
/// Deserialization accepts both V1 and the unversioned top-level form emitted
/// before V1.
///
/// # Examples
///
/// ```rust
/// use qubit_config::Config;
///
/// let mut config = Config::new();
///
/// // Set configuration values (type inference)
/// config.set("port", 8080).unwrap();                    // inferred as i32
/// config.set("host", "localhost").unwrap();
/// // &str is converted to String
/// config.set("debug", true).unwrap();                   // inferred as bool
/// config.set("timeout", 30.5).unwrap();                 // inferred as f64
/// config.set("code", 42u8).unwrap();                    // inferred as u8
///
/// // Set multiple values (type inference)
/// config.set("ports", vec![8080, 8081, 8082]).unwrap(); // inferred as i32
/// config.set("hosts", vec!["host1", "host2"]).unwrap();
/// // &str elements are converted
///
/// // Read configuration values (type inference)
/// let port: i32 = config.get("port").unwrap();
/// let host: String = config.get("host").unwrap();
/// let debug: bool = config.get("debug").unwrap();
/// let code: u8 = config.get("code").unwrap();
///
/// // Read configuration values (turbofish)
/// let port = config.get::<i32>("port").unwrap();
///
/// // Read configuration value or use default
/// let timeout: f64 = config.get_or("timeout", 30.0).unwrap();
/// ```
#[must_use]
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Configuration description
    description: Option<String>,
    /// Configuration property mapping
    pub(crate) properties: HashMap<String, Property>,
    /// Runtime read parsing options
    read_options: ReadOptions,
}

impl Serialize for Config {
    /// Serializes this configuration through the stable V1 persistence format.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ConfigWireV1Ref::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Config {
    /// Deserializes either the stable V1 format or the legacy unversioned form.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigWire::deserialize(deserializer)
            .and_then(|wire| Self::try_from(wire).map_err(D::Error::custom))
    }
}

impl TryFrom<ConfigSerdeRepr> for Config {
    type Error = String;

    /// Builds a configuration only when every map key matches its property.
    ///
    /// # Parameters
    ///
    /// * `value` - Deserialized wire representation to validate.
    ///
    /// # Returns
    ///
    /// A configuration whose property-name invariant is satisfied.
    ///
    /// # Errors
    ///
    /// Returns an error naming the first mismatched map key and property name.
    fn try_from(value: ConfigSerdeRepr) -> Result<Self, Self::Error> {
        Self::from_wire_parts(
            value.description,
            value.properties,
            value.read_options,
        )
    }
}

impl TryFrom<ConfigWireV1> for Config {
    type Error = String;

    /// Builds a configuration from the stable V1 persistence format.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire version is unsupported or a map key does
    /// not match its embedded property name.
    fn try_from(value: ConfigWireV1) -> Result<Self, Self::Error> {
        if value.version != 1 {
            return Err(format!(
                "unsupported config wire version {}; expected 1",
                value.version,
            ));
        }
        Self::from_wire_parts(
            value.description,
            value.properties.into_iter().collect(),
            value.read_options,
        )
    }
}

impl TryFrom<ConfigWire> for Config {
    type Error = String;

    /// Builds a configuration from any accepted persisted representation.
    ///
    /// # Errors
    ///
    /// Returns the validation failure for the selected wire representation.
    fn try_from(value: ConfigWire) -> Result<Self, Self::Error> {
        match value {
            ConfigWire::V1(value) => Self::try_from(value),
            ConfigWire::Legacy(value) => Self::try_from(value),
        }
    }
}

impl Config {
    /// Validates shared fields decoded from an accepted persisted wire format.
    fn from_wire_parts(
        description: Option<String>,
        properties: HashMap<String, Property>,
        read_options: ReadOptions,
    ) -> Result<Self, String> {
        if let Some((key, violation)) = properties
            .keys()
            .filter_map(|key| {
                crate::config_path::validate_config_key(key)
                    .err()
                    .map(|violation| (key, violation))
            })
            .min_by_key(|(key, _)| *key)
        {
            return Err(ConfigError::InvalidKey {
                key: key.clone(),
                violation,
            }
            .to_string());
        }
        if let Some((key, property)) = properties
            .iter()
            .filter(|(key, property)| key.as_str() != property.name())
            .min_by_key(|(key, _)| *key)
        {
            return Err(format!(
                "configuration map key '{key}' does not match property name '{}'",
                property.name(),
            ));
        }
        Ok(Self {
            description,
            properties,
            read_options,
        })
    }
    /// Creates a new empty configuration
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// assert!(config.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            description: None,
            properties: HashMap::new(),
            read_options: ReadOptions::default(),
        }
    }

    /// Creates a configuration with description
    ///
    /// # Parameters
    ///
    /// * `description` - Configuration description
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::with_description("Server Configuration");
    /// assert_eq!(config.description(), Some("Server Configuration"));
    /// ```
    #[inline]
    pub fn with_description(description: &str) -> Self {
        Self {
            description: Some(description.to_string()),
            properties: HashMap::new(),
            read_options: ReadOptions::default(),
        }
    }

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
    /// transformation options.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Prefix used to select environment variables.
    /// * `strip_prefix` - Whether to strip the prefix from loaded keys.
    /// * `convert_underscores` - Whether to convert underscores to dots.
    /// * `lowercase_keys` - Whether to lowercase loaded keys.
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
    pub fn from_env_options(
        prefix: &str,
        strip_prefix: bool,
        convert_underscores: bool,
        lowercase_keys: bool,
    ) -> ConfigResult<Self> {
        let source = EnvConfigSource::with_options(
            prefix,
            strip_prefix,
            convert_underscores,
            lowercase_keys,
        );
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

    // ========================================================================
    // Basic Property Access
    // ========================================================================

    /// Gets the configuration description
    ///
    /// # Returns
    ///
    /// Returns the configuration description as Option
    #[inline(always)]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the configuration description
    ///
    /// # Parameters
    ///
    /// * `description` - Configuration description
    ///
    /// # Returns
    ///
    /// Nothing.
    #[inline(always)]
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Gets the global read parsing options.
    ///
    /// # Returns
    ///
    /// The options used by `get`, `get_any`, and field reads when no
    /// field-level override is provided.
    #[inline(always)]
    pub fn read_options(&self) -> &ReadOptions {
        &self.read_options
    }

    /// Sets the global read parsing options.
    ///
    /// # Parameters
    ///
    /// * `options` - New read parsing options.
    ///
    /// # Returns
    ///
    /// Mutable reference to this configuration for chaining.
    #[inline(always)]
    pub fn set_read_options(&mut self, options: ReadOptions) -> &mut Self {
        self.read_options = options;
        self
    }

    /// Returns this configuration with different read parsing options.
    ///
    /// # Parameters
    ///
    /// * `options` - Read options for the returned configuration.
    ///
    /// # Returns
    ///
    /// This [`Config`] using `options`.
    #[inline]
    pub fn with_read_options(mut self, options: ReadOptions) -> Self {
        self.read_options = options;
        self
    }

    /// Creates a read-only section rooted at `path`.
    ///
    /// Property names read through the returned section are interpreted
    /// strictly relative to its canonical path.
    ///
    /// # Arguments
    ///
    /// * `path` - Canonical root-relative section path. An empty path
    ///   represents the root.
    ///
    /// # Returns
    ///
    /// A section borrowing this configuration.
    #[inline(always)]
    pub fn section(&self, path: &str) -> ConfigResult<ConfigSection<'_>> {
        ConfigSection::new(self, path)
    }

    // ========================================================================
    // Configuration Item Management
    // ========================================================================

    /// Checks if the configuration contains an item with the specified name
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns `true` if the configuration item exists
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    ///
    /// assert!(config.contains("port").unwrap());
    /// assert!(!config.contains("host").unwrap());
    /// ```
    #[inline]
    pub fn contains(&self, name: impl ConfigName) -> ConfigResult<bool> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            Ok(self.properties.contains_key(name))
        })
    }

    /// Gets a reference to a configuration item
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns Option containing the configuration item
    #[inline]
    pub fn get_property(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<&Property>> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            Ok(self.properties.get(name))
        })
    }

    /// Gets guarded mutable access to a non-final configuration item.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(_))` for an existing non-final property, `Ok(None)`
    /// for a missing property, or [`ConfigError::PropertyIsFinal`] for an
    /// existing final property. The returned guard re-checks final state before
    /// each value-changing operation.
    #[inline]
    pub fn get_property_mut(
        &mut self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<ConfigPropertyMut<'_>>> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            self.ensure_property_not_final(name)?;
            Ok(self.properties.get_mut(name).map(ConfigPropertyMut::new))
        })
    }

    /// Sets the final flag of an existing configuration item.
    ///
    /// A non-final property can be marked final. A property that is already
    /// final may be marked final again, but cannot be unset through this API.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    /// * `is_final` - Whether the property should be final.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::PropertyNotFound`] if the key does not exist.
    /// - [`ConfigError::PropertyIsFinal`] when trying to unset a final
    ///   property.
    pub fn set_final(
        &mut self,
        name: impl ConfigName,
        is_final: bool,
    ) -> ConfigResult<()> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            let property = self.properties.get_mut(name).ok_or_else(|| {
                ConfigError::PropertyNotFound(name.to_string())
            })?;
            if property.is_final() && !is_final {
                return Err(ConfigError::PropertyIsFinal(name.to_string()));
            }
            property.set_final(is_final);
            Ok(())
        })
    }

    /// Removes a non-final configuration item.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns the removed configuration item, or None if it doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    ///
    /// let removed = config.remove("port").unwrap();
    /// assert!(removed.is_some());
    /// assert!(!config.contains("port").unwrap());
    /// ```
    #[inline]
    pub fn remove(
        &mut self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<Property>> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            self.ensure_property_not_final(name)?;
            Ok(self.properties.remove(name))
        })
    }

    /// Clears all configuration items if none of them are final.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    /// config.set("host", "localhost").unwrap();
    ///
    /// config.clear().unwrap();
    /// assert!(config.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(())` when all properties were removed.
    #[inline]
    pub fn clear(&mut self) -> ConfigResult<()> {
        self.ensure_no_final_properties()?;
        self.properties.clear();
        Ok(())
    }

    /// Gets the number of configuration items
    ///
    /// # Returns
    ///
    /// Returns the number of configuration items
    #[inline]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Checks if the configuration is empty
    ///
    /// # Returns
    ///
    /// Returns `true` if the configuration contains no items
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Gets all configuration item names
    ///
    /// # Returns
    ///
    /// Returns a Vec of configuration item names
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    /// config.set("host", "localhost").unwrap();
    ///
    /// let keys = config.keys();
    /// assert_eq!(keys.len(), 2);
    /// assert!(keys.contains(&"port".to_string()));
    /// assert!(keys.contains(&"host".to_string()));
    /// ```
    pub fn keys(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    // ========================================================================
    // Core Generic Methods
    // ========================================================================

    /// Gets a configuration value, converting the stored first value to `T`.
    ///
    /// Core read API with type inference.
    ///
    /// String-backed values are converted without interpolating placeholders.
    /// Use [`Self::get_interpolated`] for explicit interpolation.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`]
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// The value of the specified type on success, or a [`ConfigError`] on
    /// failure.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::PropertyNotFound`] if the key does not exist
    /// - [`ConfigError::PropertyHasNoValue`] if the property has no value
    /// - [`ConfigError::ConversionError`] if the stored value cannot be
    ///   converted to `T`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    /// config.set("host", "localhost").unwrap();
    ///
    /// // Method 1: Type inference
    /// let port: i32 = config.get("port").unwrap();
    /// let host: String = config.get("host").unwrap();
    ///
    /// // Method 2: Turbofish
    /// let port = config.get::<i32>("port").unwrap();
    /// let host = config.get::<String>("host").unwrap();
    ///
    /// // Method 3: Inference from usage
    /// fn start_server(port: i32, host: String) { }
    /// start_server(config.get("port").unwrap(), config.get("host").unwrap());
    /// ```
    pub fn get<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get(self, name)
    }

    /// Gets a configuration value after interpolating string-backed values.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    ///
    /// # Returns
    ///
    /// The interpolated and converted value.
    ///
    /// # Errors
    ///
    /// Returns missing-value, interpolation, resource-limit, or conversion
    /// errors with key context.
    #[inline(always)]
    pub fn get_interpolated<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_interpolated(self, name)
    }

    /// Gets a configuration value only when the stored value already has the
    /// exact requested type.
    ///
    /// Unlike [`Self::get`], this method preserves the pre-conversion read
    /// semantics. For example, a stored string `"1"` can be read as `bool` by
    /// [`Self::get`], but [`Self::get_strict`] returns
    /// [`ConfigError::TypeMismatch`].
    ///
    /// # Type Parameters
    ///
    /// * `T` - Exact target type supported by both value shapes.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// The exact typed value on success, or a [`ConfigError`] on failure.
    pub fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: StrictValueRead,
    {
        name.with_config_name(|name| {
            let property = self.get_property_by_name(name)?;

            property
                .get_first::<T>()
                .map_err(|e| utils::map_value_error(name, e))
        })
    }

    /// Gets a configuration value or returns a default value.
    ///
    /// Returns `default` only if the key is missing or explicitly empty.
    /// Conversion errors are returned.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`]
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value
    ///
    /// # Returns
    ///
    /// Returns the configuration value or default value. Conversion errors are
    /// returned instead of being hidden by the default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::new();
    ///
    /// let port: i32 = config.get_or("port", 8080).unwrap();
    /// let host: String = config.get_or("host", "localhost").unwrap();
    ///
    /// assert_eq!(port, 8080);
    /// assert_eq!(host, "localhost");
    /// ```
    pub fn get_or<T>(
        &self,
        name: impl ConfigName,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_or(self, name, default)
    }

    /// Gets an interpolated configuration value or a typed default.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    /// * `default` - Fallback used only when the value is absent or missing.
    ///
    /// # Returns
    ///
    /// The interpolated value or supplied default.
    ///
    /// # Errors
    ///
    /// Returns interpolation and conversion errors instead of hiding them
    /// behind the default.
    #[inline(always)]
    pub fn get_interpolated_or<T>(
        &self,
        name: impl ConfigName,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_interpolated_or(self, name, default)
    }

    /// Gets the first configured value from `names`.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// Parsed value from the first present and non-empty key.
    pub fn get_any<T>(&self, names: impl ConfigNames) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_any(self, names)
    }

    /// Gets the first configured value after interpolation.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// The interpolated value from the first present, non-empty key.
    ///
    /// # Errors
    ///
    /// Returns missing-value, interpolation, resource-limit, or conversion
    /// errors.
    #[inline(always)]
    pub fn get_any_interpolated<T>(
        &self,
        names: impl ConfigNames,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_any_interpolated(self, names)
    }

    /// Gets an optional value from the first configured key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` when every key is absent or effectively missing.
    pub fn get_optional_any<T>(
        &self,
        names: impl ConfigNames,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional_any(self, names)
    }

    /// Gets an optional interpolated value from the first configured key.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// `Some` for the first configured value or `None` when every candidate is
    /// absent or effectively missing.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors.
    #[inline(always)]
    pub fn get_optional_any_interpolated<T>(
        &self,
        names: impl ConfigNames,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional_any_interpolated(self, names)
    }

    /// Gets the first configured value from `names`, or `default` when absent.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    /// * `default` - Fallback used only when every key is absent or effectively
    ///   missing.
    ///
    /// # Returns
    ///
    /// Parsed value or `default`; conversion errors are returned.
    pub fn get_any_or<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_any_or(self, names, default)
    }

    /// Gets the first interpolated value from `names`, or `default` when all
    /// candidates are absent.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    /// * `default` - Fallback used only when every key is absent or effectively
    ///   missing.
    /// # Returns
    ///
    /// Interpolated value or `default`.
    ///
    /// # Errors
    ///
    /// Returns interpolation and conversion errors instead of hiding them
    /// behind the default.
    #[inline(always)]
    pub fn get_any_interpolated_or<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_any_interpolated_or(self, names, default)
    }

    /// Reads a declared configuration field.
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration with name, aliases, defaults, and optional
    ///   read options.
    ///
    /// # Returns
    ///
    /// Parsed field value or default.
    pub fn read<T>(&self, field: ConfigField<T>) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::read(self, field)
    }

    /// Reads an optional declared configuration field.
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration.
    ///
    /// # Returns
    ///
    /// Parsed field value, default, or `None`.
    pub fn read_optional<T>(
        &self,
        field: ConfigField<T>,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::read_optional(self, field)
    }

    /// Reads a declared field after interpolating string-backed values.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration with names, a default, and optional read
    ///   options.
    ///
    /// # Returns
    ///
    /// Interpolated field value or its typed default.
    ///
    /// # Errors
    ///
    /// Returns missing-value, interpolation, resource-limit, or conversion
    /// errors.
    #[inline(always)]
    pub fn read_interpolated<T>(&self, field: ConfigField<T>) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::read_interpolated(self, field)
    }

    /// Reads an optional declared field after interpolation.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration with names, a default, and optional read
    ///   options.
    ///
    /// # Returns
    ///
    /// Interpolated field value, its typed default, or `None`.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors.
    #[inline(always)]
    pub fn read_optional_interpolated<T>(
        &self,
        field: ConfigField<T>,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::read_optional_interpolated(self, field)
    }

    /// Gets a list of configuration values, converting each stored element to
    /// `T`.
    ///
    /// Gets all values of a configuration item (multi-value configuration).
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`]
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns a list of values on success, or an error on failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("ports", vec![8080, 8081, 8082]).unwrap();
    ///
    /// let ports: Vec<i32> = config.get_list("ports").unwrap();
    /// assert_eq!(ports, vec![8080, 8081, 8082]);
    /// ```
    pub fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: DataConversionTarget,
    {
        <Self as ConfigReader>::get(self, name)
    }

    /// Gets all configuration values only when the stored values already have
    /// the exact requested element type.
    ///
    /// Unlike [`Self::get_list`], this method preserves the pre-conversion
    /// list read semantics. It returns an empty vector for empty properties and
    /// [`ConfigError::TypeMismatch`] for non-empty values of another stored
    /// type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Exact element type supported by both value shapes.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// A vector of exact typed values on success, or a [`ConfigError`] on
    /// failure.
    pub fn get_list_strict<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Vec<T>>
    where
        T: StrictValueListRead,
    {
        name.with_config_name(|name| {
            let property = self.get_property_by_name(name)?;
            property
                .get_list::<T>()
                .map_err(|e| utils::map_value_error(name, e))
        })
    }

    /// Sets a configuration value
    ///
    /// This is the core method for setting configuration values, supporting
    /// type inference.
    ///
    /// # Type Parameters
    ///
    /// * `S` - Input value container type inferred from `values`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `values` - Scalar or collection accepted by [`ValueContainer`].
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error on failure
    ///
    /// # Errors
    ///
    /// - [`ConfigError::PropertyIsFinal`] if the property is marked final
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    ///
    /// // Set single values (type auto-inference)
    /// config.set("port", 8080).unwrap();                    // scalar i32
    /// config.set("host", "localhost").unwrap();
    /// // &str is converted
    /// config.set("debug", true).unwrap();                   // scalar bool
    /// config.set("timeout", 30.5).unwrap();                 // scalar f64
    ///
    /// // Set multiple values (type auto-inference)
    /// config.set("ports", vec![8080, 8081, 8082]).unwrap(); // i32 collection
    /// config.set("hosts", vec!["host1", "host2"]).unwrap();
    /// // &str collection (then converted)
    /// ```
    pub fn set<S>(
        &mut self,
        name: impl ConfigName,
        values: S,
    ) -> ConfigResult<()>
    where
        S: Into<ValueContainer>,
    {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            self.ensure_property_not_final(name)?;
            let value = values.into();
            if let Some(property) = self.properties.get_mut(name) {
                property.set(value);
            } else {
                let property = Property::new(name, value)?;
                self.properties.insert(name.to_string(), property);
            }
            Ok(())
        })
    }

    /// Adds configuration values
    ///
    /// Adds values to an existing configuration item (multi-value properties).
    ///
    /// # Type Parameters
    ///
    /// * `S` - Input value container type inferred from `values`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `values` - Values to append; supports the same forms as [`Self::set`]
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error on failure
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] for a final property, or a
    /// value error when appended values are incompatible with existing ones.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();                    // Set initial value
    /// config.add("port", 8081).unwrap();                    // Add single value
    /// config.add("port", vec![8082, 8083]).unwrap();        // Add multiple values
    /// config.add("port", vec![8084, 8085]).unwrap();       // Add slice
    ///
    /// let ports: Vec<i32> = config.get_list("port").unwrap();
    /// assert_eq!(ports, vec![8080, 8081, 8082, 8083, 8084, 8085]);
    /// ```
    pub fn add<S>(
        &mut self,
        name: impl ConfigName,
        values: S,
    ) -> ConfigResult<()>
    where
        S: Into<ValueContainer>,
    {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            self.ensure_property_not_final(name)?;
            let value = values.into();
            if let Some(property) = self.properties.get_mut(name) {
                property
                    .add(value)
                    .map_err(|error| ConfigError::from((name, error)))
            } else {
                let property = Property::new(name, value)?;
                self.properties.insert(name.to_string(), property);
                Ok(())
            }
        })
    }

    /// Merges configuration from a `ConfigSource`
    ///
    /// Loads all key-value pairs from the given source and merges them into
    /// this configuration. Existing non-final properties are overwritten;
    /// final properties are preserved and cause an error if the source tries
    /// to overwrite them.
    ///
    /// # Parameters
    ///
    /// * `source` - The configuration source to load from
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `ConfigError` on failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "toml")]
    /// # {
    /// use qubit_config::Config;
    /// use qubit_config::source::{
    ///     CompositeConfigSource, ConfigSource,
    ///     EnvConfigSource, TomlConfigSource,
    /// };
    ///
    /// let mut composite = CompositeConfigSource::new();
    /// let path = std::env::temp_dir().join(format!(
    ///     "qubit-config-doc-{}.toml",
    ///     std::process::id()
    /// ));
    /// std::fs::write(&path, "app.name = \"demo\"").unwrap();
    /// composite.add(TomlConfigSource::from_file(&path));
    /// composite.add(EnvConfigSource::with_prefix("APP_"));
    ///
    /// let mut config = Config::new();
    /// config.merge_from_source(&composite).unwrap();
    /// std::fs::remove_file(&path).unwrap();
    /// # }
    /// ```
    #[inline]
    pub fn merge_from_source(
        &mut self,
        source: &dyn ConfigSource,
    ) -> ConfigResult<()> {
        let mut staged = self.clone();
        source.load_into(&mut staged)?;
        *self = staged;
        Ok(())
    }

    // ========================================================================
    // Prefix Traversal and Sub-tree Extraction (v0.4.0)
    // ========================================================================

    /// Iterates over all configuration entries as `(key, &Property)` pairs.
    ///
    /// # Returns
    ///
    /// An iterator yielding `(&str, &Property)` tuples.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("host", "localhost").unwrap();
    /// config.set("port", 8080).unwrap();
    ///
    /// for (key, prop) in config.iter() {
    ///     println!("{} = {:?}", key, prop);
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Property)> {
        self.properties.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterates over all configuration entries whose key starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to filter by (e.g., `"http."`)
    ///
    /// # Returns
    ///
    /// An iterator of `(&str, &Property)` whose keys start with `prefix`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost").unwrap();
    /// config.set("http.port", 8080).unwrap();
    /// config.set("db.host", "dbhost").unwrap();
    ///
    /// let http_entries: Vec<_> = config.iter_prefix("http.").collect();
    /// assert_eq!(http_entries.len(), 2);
    /// ```
    #[inline]
    pub fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Property)> {
        self.properties
            .iter()
            .filter(move |(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Returns `true` if any configuration key starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to check
    ///
    /// # Returns
    ///
    /// `true` if at least one key starts with `prefix`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost").unwrap();
    ///
    /// assert!(config.contains_key_prefix("http."));
    /// assert!(!config.contains_key_prefix("db."));
    /// ```
    #[inline]
    pub fn contains_key_prefix(&self, prefix: &str) -> bool {
        self.properties.keys().any(|k| k.starts_with(prefix))
    }

    /// Returns whether a dotted configuration section has descendants.
    ///
    /// An exact scalar at `path` and sibling names such as `proxy2` do not
    /// make the `proxy` section present.
    ///
    /// # Parameters
    ///
    /// * `path` - Canonical dotted section path. An empty path represents the
    ///   root.
    ///
    /// # Returns
    ///
    /// `true` when at least one descendant key belongs to the section.
    #[inline]
    pub fn contains_section(&self, path: &str) -> ConfigResult<bool> {
        ensure_config_path(path)?;
        if path.is_empty() {
            return Ok(!self.properties.is_empty());
        }
        let child_prefix = format!("{path}.");
        Ok(self.contains_key_prefix(&child_prefix))
    }

    /// Extracts a sub-configuration for child keys below `prefix`.
    ///
    /// An exact key equal to `prefix` is treated as a value, not as part of the
    /// extracted subtree. For example, `subconfig("http", true)` includes
    /// `http.host` as `host`, but does not include an exact `http` property.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to extract (e.g., `"http"`)
    /// * `strip_prefix` - When `true`, removes `prefix` and the following dot
    ///   from keys in the result; when `false`, keys are copied unchanged.
    ///
    /// # Returns
    ///
    /// A new `Config` containing only child entries below `prefix`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost").unwrap();
    /// config.set("http.port", 8080).unwrap();
    /// config.set("db.host", "dbhost").unwrap();
    ///
    /// let http_config = config.subconfig("http", true).unwrap();
    /// assert!(http_config.contains("host").unwrap());
    /// assert!(http_config.contains("port").unwrap());
    /// assert!(!http_config.contains("db.host").unwrap());
    /// ```
    pub fn subconfig(
        &self,
        prefix: &str,
        strip_prefix: bool,
    ) -> ConfigResult<Config> {
        ensure_config_path(prefix)?;
        let mut sub = Config::new();
        sub.description = self.description.clone();
        sub.read_options = self.read_options.clone();

        // Empty prefix means "all keys"
        if prefix.is_empty() {
            for (k, v) in &self.properties {
                sub.properties.insert(k.clone(), v.clone());
            }
            return Ok(sub);
        }

        let full_prefix = format!("{prefix}.");

        for (k, v) in &self.properties {
            if k.starts_with(&full_prefix) {
                let new_key = if strip_prefix {
                    k[full_prefix.len()..].to_string()
                } else {
                    k.clone()
                };
                let property = if strip_prefix {
                    v.renamed(new_key.clone())
                } else {
                    v.clone()
                };
                sub.properties.insert(new_key, property);
            }
        }

        Ok(sub)
    }

    // ========================================================================
    // Optional and Null Semantics (v0.4.0)
    // ========================================================================

    /// Returns `true` if the property exists but has no value (empty / null).
    ///
    /// This distinguishes between:
    /// - Key does not exist → `contains()` returns `false`
    /// - Key exists but is empty/null → `is_null()` returns `true`
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `true` if the property exists and has zero values. This includes both
    /// an unset property and a concrete empty collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    /// use qubit_datatype::DataType;
    ///
    /// let mut config = Config::new();
    /// config.set_null("nullable", DataType::String).unwrap();
    ///
    /// assert!(config.is_null("nullable").unwrap());
    /// assert!(!config.is_null("missing").unwrap());
    /// ```
    pub fn is_null(&self, name: impl ConfigName) -> ConfigResult<bool> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            Ok(self
                .properties
                .get(name)
                .map(|p| p.is_empty())
                .unwrap_or(false))
        })
    }

    /// Gets an optional configuration value.
    ///
    /// Distinguishes between three states:
    /// - `Ok(Some(value))` – key exists and has a value
    /// - `Ok(None)` – key does not exist or is effectively missing
    /// - `Err(e)` – key exists and has a value, but conversion failed
    ///
    /// Concrete empty collections remain present and deserialize as an empty
    /// collection when `T` is a collection type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `Ok(Some(value))`, `Ok(None)`, or `Err` as described above.
    ///
    /// # Errors
    ///
    /// Returns conversion errors for configured values that cannot be read as
    /// `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080).unwrap();
    ///
    /// let port: Option<i32> = config.get_optional("port").unwrap();
    /// assert_eq!(port, Some(8080));
    ///
    /// let missing: Option<i32> = config.get_optional("missing").unwrap();
    /// assert_eq!(missing, None);
    /// ```
    pub fn get_optional<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional(self, name)
    }

    /// Gets an optional value after interpolating string-backed values.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type supported by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    ///
    /// # Returns
    ///
    /// `Ok(Some(value))` for a configured value, `Ok(None)` when absent or
    /// effectively missing after interpolation, or `Err` on failure.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors with key
    /// context.
    #[inline(always)]
    pub fn get_optional_interpolated<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional_interpolated(self, name)
    }

    /// Gets an optional list of configuration values.
    ///
    /// String elements are converted without interpolating placeholders.
    ///
    /// Distinguishes between three states:
    /// - `Ok(Some(vec))` – key exists and has values
    /// - `Ok(None)` – key does not exist or is effectively missing
    /// - `Err(e)` – key exists and has values, but conversion failed
    ///
    /// A concrete empty collection returns `Ok(Some(Vec::new()))`.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target element type supported by [`DataConversionTarget`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, `Ok(None)`, or `Err` as described above.
    ///
    /// # Errors
    ///
    /// Returns conversion errors for configured list elements that cannot be
    /// read as `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("ports", vec![8080, 8081]).unwrap();
    ///
    /// let ports: Option<Vec<i32>> = config.get_optional_list("ports").unwrap();
    /// assert_eq!(ports, Some(vec![8080, 8081]));
    ///
    /// let missing: Option<Vec<i32>> = config.get_optional_list("missing").unwrap();
    /// assert_eq!(missing, None);
    /// ```
    pub fn get_optional_list<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<Vec<T>>>
    where
        T: DataConversionTarget,
    {
        <Self as ConfigReader>::get_optional(self, name)
    }

    // ========================================================================
    // Structured Config Deserialization (v0.4.0)
    // ========================================================================

    /// Deserializes the subtree at `prefix` through the configuration Serde
    /// view.
    ///
    /// String-backed scalar and collection projections apply this config's
    /// [`ReadOptions`] conversion policies without interpolating placeholders.
    /// Use [`Self::deserialize_interpolated`] for explicit interpolation.
    ///
    /// The Serde view is JSON-like: mappings, sequences, booleans, strings,
    /// numbers, and null values are exposed according to Serde's data model.
    /// This method does not promise the native conversion shapes used by
    /// [`Self::get`] for rich values such as durations or arbitrary-precision
    /// integers.
    ///
    /// When `prefix` is non-empty, an exact property named `prefix` is
    /// deserialized as the root value. If no exact property exists, child keys
    /// under `prefix` (prefix and trailing dot removed) form an object for
    /// `serde`, for example:
    ///
    /// ```rust
    /// #[derive(serde::Deserialize)]
    /// struct HttpOptions {
    ///     host: String,
    ///     port: u16,
    /// }
    /// ```
    ///
    /// can be populated from config keys `http.host` and `http.port` by calling
    /// `config.deserialize::<HttpOptions>("http")`. Defining both `http` and
    /// `http.*` is a [`ConfigError::KeyConflict`], as are ambiguous dotted
    /// paths such as `a` and `a.b` inside the same deserialized object.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix for the struct fields (`""` means the root map)
    ///
    /// # Returns
    ///
    /// The deserialized `T`.
    ///
    /// # Errors
    ///
    /// Returns the original [`ConfigError`] when configuration lookup or
    /// conversion fails, preserving its kind, leaf path, and source. A
    /// mismatch raised only by `T`'s Serde implementation returns
    /// [`ConfigError::DeserializeError`] at `prefix` with a fixed sanitized
    /// message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Server {
    ///     host: String,
    ///     port: i32,
    /// }
    ///
    /// let mut config = Config::new();
    /// config.set("server.host", "localhost").unwrap();
    /// config.set("server.port", 8080).unwrap();
    ///
    /// let server: Server = config.deserialize("server").unwrap();
    /// assert_eq!(server.host, "localhost");
    /// assert_eq!(server.port, 8080);
    /// ```
    pub fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize(self, prefix)
    }

    /// Deserializes an exact value or subtree after interpolating strings.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type implementing `serde::de::DeserializeOwned`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Exact property key or dotted subtree path; an empty string
    ///   selects the root map.
    ///
    /// # Returns
    ///
    /// The interpolated and deserialized value.
    ///
    /// # Errors
    ///
    /// Returns lookup, interpolation, resource-limit, conversion, key-conflict,
    /// or sanitized Serde errors with their original configuration path.
    pub fn deserialize_interpolated<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize_interpolated(self, prefix)
    }

    /// Inserts or replaces a property using an explicit [`Property`] object.
    ///
    /// This method enforces two invariants:
    ///
    /// - `name` must exactly match `property.name()`
    /// - existing final properties cannot be overridden
    ///
    /// # Parameters
    ///
    /// * `name` - Target key in this config.
    /// * `property` - Property to store under `name`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::MergeError`] when `name` and `property.name()` differ.
    /// - [`ConfigError::PropertyIsFinal`] when trying to override a final
    ///   property.
    pub fn insert_property(
        &mut self,
        name: impl ConfigName,
        property: Property,
    ) -> ConfigResult<()> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
            if property.name() != name {
                return Err(ConfigError::MergeError(format!(
                    "Property name mismatch: key '{name}' != property '{}'",
                    property.name()
                )));
            }
            self.ensure_property_not_final(name)?;
            self.properties.insert(name.to_string(), property);
            Ok(())
        })
    }

    /// Sets a key to a typed null/empty value.
    ///
    /// This is the preferred public API for representing null/empty values
    /// without exposing raw mutable access to the internal map.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    /// * `data_type` - Data type metadata for the empty value.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::PropertyIsFinal`] when trying to override a final
    ///   property.
    #[inline]
    pub fn set_null(
        &mut self,
        name: impl ConfigName,
        data_type: DataType,
    ) -> ConfigResult<()> {
        name.with_config_name(|name| {
            let property = Property::new(
                name,
                ValueContainer::Scalar(QubitValue::new_unset(data_type)),
            )?;
            self.insert_property(name, property)
        })
    }

    /// Looks up a property by key for internal read paths.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(&Property)` if the key exists, or [`ConfigError::PropertyNotFound`]
    /// otherwise.
    #[inline]
    fn get_property_by_name(&self, name: &str) -> ConfigResult<&Property> {
        ensure_config_key(name)?;
        self.properties
            .get(name)
            .ok_or_else(|| ConfigError::PropertyNotFound(name.to_string()))
    }

    /// Ensures the entry for `name` is not marked final before a write.
    ///
    /// Missing keys are allowed because writes may create them.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the key is absent or not final.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if an existing property is
    /// final.
    #[inline]
    fn ensure_property_not_final(&self, name: &str) -> ConfigResult<()> {
        if let Some(prop) = self.properties.get(name)
            && prop.is_final()
        {
            return Err(ConfigError::PropertyIsFinal(name.to_string()));
        }
        Ok(())
    }

    /// Ensures no property is final before a bulk destructive operation.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every property is mutable.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] for the first final property.
    #[inline]
    fn ensure_no_final_properties(&self) -> ConfigResult<()> {
        if let Some((name, _)) =
            self.properties.iter().find(|(_, prop)| prop.is_final())
        {
            return Err(ConfigError::PropertyIsFinal(name.clone()));
        }
        Ok(())
    }
}

impl ConfigReader for Config {
    #[inline]
    fn read_options(&self) -> &ReadOptions {
        Config::read_options(self)
    }

    #[inline]
    fn get_property(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<&Property>> {
        Config::get_property(self, name)
    }

    #[inline]
    fn len(&self) -> usize {
        Config::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        Config::is_empty(self)
    }

    #[inline]
    fn keys(&self) -> Vec<String> {
        Config::keys(self)
    }

    #[inline]
    fn contains(&self, name: impl ConfigName) -> ConfigResult<bool> {
        Config::contains(self, name)
    }

    #[inline]
    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: StrictValueRead,
    {
        Config::get_strict(self, name)
    }

    #[inline]
    fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: DataConversionTarget,
    {
        Config::get_list(self, name)
    }

    #[inline]
    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: StrictValueListRead,
    {
        Config::get_list_strict(self, name)
    }

    #[inline]
    fn get_optional_list<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<Vec<T>>>
    where
        T: DataConversionTarget,
    {
        Config::get_optional_list(self, name)
    }

    #[inline]
    fn contains_key_prefix(&self, prefix: &str) -> bool {
        Config::contains_key_prefix(self, prefix)
    }

    #[inline]
    fn contains_section(&self, path: &str) -> ConfigResult<bool> {
        Config::contains_section(self, path)
    }

    #[inline]
    fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a> {
        Box::new(Config::iter_prefix(self, prefix))
    }

    #[inline]
    fn iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a> {
        Box::new(Config::iter(self))
    }

    #[inline]
    fn is_null(&self, name: impl ConfigName) -> ConfigResult<bool> {
        Config::is_null(self, name)
    }

    #[inline]
    fn section(&self, path: &str) -> ConfigResult<ConfigSection<'_>> {
        Config::section(self, path)
    }
}

impl Default for Config {
    /// Creates a new default configuration
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.is_empty());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
