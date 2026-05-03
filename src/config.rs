/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Manager
//!
//! Provides storage, retrieval, and management of configurations.
//!

#![allow(private_bounds)]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::path::Path;

use crate::ConfigPropertyMut;
use crate::config_prefix_view::ConfigPrefixView;
use crate::config_reader::ConfigReader;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_DEPTH;
use crate::field::ConfigField;
use crate::from::{FromConfig, IntoConfigDefault};
use crate::options::ConfigReadOptions;
use crate::source::{
    ConfigSource, EnvConfigSource, EnvFileConfigSource, PropertiesConfigSource, TomlConfigSource,
    YamlConfigSource,
};
use crate::utils;
use crate::{ConfigError, ConfigName, ConfigNames, ConfigResult, Property};
use qubit_datatype::{DataConvertTo, DataConverter, DataType};
use qubit_value::multi_values::{
    MultiValuesAddArg, MultiValuesAdder, MultiValuesFirstGetter, MultiValuesGetter,
    MultiValuesMultiAdder, MultiValuesSetArg, MultiValuesSetter, MultiValuesSetterSlice,
    MultiValuesSingleSetter,
};
use qubit_value::{MultiValues, Value as QubitValue};

pub(crate) fn convert_deserialize_number<T>(
    key: &str,
    options: &ConfigReadOptions,
    value: String,
) -> ConfigResult<T>
where
    for<'a> DataConverter<'a>: DataConvertTo<T>,
{
    match QubitValue::String(value).to_with::<T>(options.conversion_options()) {
        Ok(value) => Ok(value),
        Err(error) => Err(ConfigError::from((key, error))),
    }
}

/// Returns `true` when `key` is strictly below `prefix`.
fn is_child_key(key: &str, prefix: &str) -> bool {
    key.len() > prefix.len()
        && key.starts_with(prefix)
        && key.as_bytes().get(prefix.len()) == Some(&b'.')
}

/// Returns whether a scalar string property is missing under deserialization options.
fn scalar_string_is_missing_for_deserialize(
    primary: &impl ConfigReader,
    fallback: &impl ConfigReader,
    key: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<bool> {
    let MultiValues::String(values) = property.value() else {
        return Ok(false);
    };
    let [value] = values.as_slice() else {
        return Ok(false);
    };
    let value = if primary.is_enable_variable_substitution() {
        utils::substitute_variables_with_fallback(
            value,
            primary,
            fallback,
            primary.max_substitution_depth(),
        )?
    } else {
        value.to_string()
    };
    match options.conversion_options().string.normalize(&value) {
        Ok(_) => Ok(false),
        Err(qubit_datatype::DataConversionError::NoValue) => Ok(true),
        Err(error) => Err(ConfigError::from_data_conversion_error(key, error)),
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
///
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Configuration description
    description: Option<String>,
    /// Configuration property mapping
    pub(crate) properties: HashMap<String, Property>,
    /// Whether variable substitution is enabled
    enable_variable_substitution: bool,
    /// Maximum depth for variable substitution
    max_substitution_depth: usize,
    /// Runtime read parsing options
    #[serde(default)]
    read_options: ConfigReadOptions,
}

impl Config {
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
            enable_variable_substitution: true,
            max_substitution_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            read_options: ConfigReadOptions::default(),
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
            enable_variable_substitution: true,
            max_substitution_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            read_options: ConfigReadOptions::default(),
        }
    }

    // ========================================================================
    // Basic Property Access
    // ========================================================================

    /// Gets the configuration description
    ///
    /// # Returns
    ///
    /// Returns the configuration description as Option
    #[inline]
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
    #[inline]
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Checks if variable substitution is enabled
    ///
    /// # Returns
    ///
    /// Returns `true` if variable substitution is enabled
    #[inline]
    pub fn is_enable_variable_substitution(&self) -> bool {
        self.enable_variable_substitution
    }

    /// Sets whether to enable variable substitution
    ///
    /// # Parameters
    ///
    /// * `enable` - Whether to enable
    ///
    /// # Returns
    ///
    /// Nothing.
    #[inline]
    pub fn set_enable_variable_substitution(&mut self, enable: bool) {
        self.enable_variable_substitution = enable;
    }

    /// Gets the maximum depth for variable substitution
    ///
    /// # Returns
    ///
    /// Returns the maximum depth value
    #[inline]
    pub fn max_substitution_depth(&self) -> usize {
        self.max_substitution_depth
    }

    /// Gets the global read parsing options.
    ///
    /// # Returns
    ///
    /// The options used by `get`, `get_any`, and field reads when no
    /// field-level override is provided.
    #[inline]
    pub fn read_options(&self) -> &ConfigReadOptions {
        &self.read_options
    }

    /// Sets the global read parsing options.
    ///
    /// # Parameters
    ///
    /// * `read_options` - New read parsing options.
    ///
    /// # Returns
    ///
    /// Mutable reference to this configuration for chaining.
    #[inline]
    pub fn set_read_options(&mut self, read_options: ConfigReadOptions) -> &mut Self {
        self.read_options = read_options;
        self
    }

    /// Returns a cloned configuration with different read parsing options.
    ///
    /// # Parameters
    ///
    /// * `read_options` - Read options for the returned configuration.
    ///
    /// # Returns
    ///
    /// A cloned [`Config`] using `read_options`.
    #[must_use]
    pub fn with_read_options(&self, read_options: ConfigReadOptions) -> Self {
        let mut config = self.clone();
        config.read_options = read_options;
        config
    }

    /// Creates a read-only prefix view using [`ConfigPrefixView`].
    ///
    /// # Parameters
    ///
    /// * `prefix` - Prefix
    ///
    /// # Returns
    ///
    /// Returns a read-only prefix view
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::{Config, ConfigReader};
    ///
    /// let mut config = Config::new();
    /// config.set("server.port", 8080).unwrap();
    /// config.set("server.host", "localhost").unwrap();
    ///
    /// let server = config.prefix_view("server");
    /// assert_eq!(server.get::<i32>("port").unwrap(), 8080);
    /// assert_eq!(server.get::<String>("host").unwrap(), "localhost");
    /// ```
    #[inline]
    pub fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'_> {
        ConfigPrefixView::new(self, prefix)
    }

    /// Sets the maximum depth for variable substitution
    ///
    /// # Parameters
    ///
    /// * `depth` - Maximum depth
    ///
    /// # Returns
    ///
    /// Nothing.
    #[inline]
    pub fn set_max_substitution_depth(&mut self, depth: usize) {
        self.max_substitution_depth = depth;
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
    /// assert!(config.contains("port"));
    /// assert!(!config.contains("host"));
    /// ```
    #[inline]
    pub fn contains(&self, name: impl ConfigName) -> bool {
        name.with_config_name(|name| self.properties.contains_key(name))
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
    pub fn get_property(&self, name: impl ConfigName) -> Option<&Property> {
        name.with_config_name(|name| self.properties.get(name))
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
    pub fn set_final(&mut self, name: impl ConfigName, is_final: bool) -> ConfigResult<()> {
        name.with_config_name(|name| {
            let property = self
                .properties
                .get_mut(name)
                .ok_or_else(|| ConfigError::PropertyNotFound(name.to_string()))?;
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
    /// assert!(!config.contains("port"));
    /// ```
    #[inline]
    pub fn remove(&mut self, name: impl ConfigName) -> ConfigResult<Option<Property>> {
        name.with_config_name(|name| {
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

    /// Looks up a property by key for internal read paths.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key
    ///
    /// # Returns
    ///
    /// `Ok(&Property)` if the key exists, or [`ConfigError::PropertyNotFound`]
    /// otherwise.
    #[inline]
    fn get_property_by_name(&self, name: &str) -> ConfigResult<&Property> {
        self.properties
            .get(name)
            .ok_or_else(|| ConfigError::PropertyNotFound(name.to_string()))
    }

    /// Ensures the entry for `name` is not marked final before a write.
    ///
    /// Missing keys are allowed (writes may create them).
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key
    ///
    /// # Returns
    ///
    /// `Ok(())` if the key is absent or not final, or
    /// [`ConfigError::PropertyIsFinal`] if an existing property is final.
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
    #[inline]
    fn ensure_no_final_properties(&self) -> ConfigResult<()> {
        if let Some((name, _)) = self.properties.iter().find(|(_, prop)| prop.is_final()) {
            return Err(ConfigError::PropertyIsFinal(name.clone()));
        }
        Ok(())
    }

    // ========================================================================
    // Core Generic Methods
    // ========================================================================

    /// Gets a configuration value, converting the stored first value to `T`.
    ///
    /// Core read API with type inference.
    ///
    /// This method does not perform `${...}` variable substitution. Use
    /// [`Self::get_string`], [`Self::get_string_list`], or
    /// [`Self::deserialize`] when placeholders should be resolved while reading.
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
    /// * `T` - Exact target type supported by [`MultiValuesFirstGetter`]
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
        MultiValues: MultiValuesFirstGetter<T>,
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
    /// Conversion and substitution errors are returned.
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
    /// Returns the configuration value or default value. Conversion and
    /// substitution errors are returned instead of being hidden by the default.
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

    /// Gets an optional value from the first configured key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` when all keys are missing or empty.
    pub fn get_optional_any<T>(&self, names: impl ConfigNames) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional_any(self, names)
    }

    /// Gets the first configured value from `names`, or `default` when absent.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    /// * `default` - Fallback used only when all keys are missing or empty.
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

    /// Gets the first configured value from `names` with explicit read options,
    /// or `default` when absent.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    /// * `default` - Fallback used only when all keys are missing or empty.
    /// * `read_options` - Parsing options for this read.
    ///
    /// # Returns
    ///
    /// Parsed value or `default`; conversion errors are returned.
    pub fn get_any_or_with<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
        read_options: &ConfigReadOptions,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_any_or_with(self, names, default, read_options)
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
    pub fn read_optional<T>(&self, field: ConfigField<T>) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::read_optional(self, field)
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
        T: FromConfig,
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
    /// * `T` - Exact element type supported by [`MultiValuesGetter`]
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// A vector of exact typed values on success, or a [`ConfigError`] on
    /// failure.
    pub fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        name.with_config_name(|name| {
            let property = self.get_property_by_name(name)?;
            if property.is_empty() {
                return Ok(Vec::new());
            }

            property
                .get::<T>()
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
    /// * `T` - Element type, automatically inferred from the `values` parameter
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `values` - Value to store; supports `T`, `Vec<T>`, `&[T]`, and related
    ///   forms accepted by [`MultiValues`] setters
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
    /// config.set("port", 8080).unwrap();                    // T inferred as i32
    /// config.set("host", "localhost").unwrap();
    /// // T inferred as String; &str is converted
    /// config.set("debug", true).unwrap();                   // T inferred as bool
    /// config.set("timeout", 30.5).unwrap();                 // T inferred as f64
    ///
    /// // Set multiple values (type auto-inference)
    /// config.set("ports", vec![8080, 8081, 8082]).unwrap(); // T inferred as i32
    /// config.set("hosts", vec!["host1", "host2"]).unwrap();
    /// // T inferred as &str (then converted)
    /// ```
    pub fn set<S>(&mut self, name: impl ConfigName, values: S) -> ConfigResult<()>
    where
        S: for<'a> MultiValuesSetArg<'a>,
        <S as MultiValuesSetArg<'static>>::Item: Clone,
        MultiValues: MultiValuesSetter<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetterSlice<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSingleSetter<<S as MultiValuesSetArg<'static>>::Item>,
    {
        name.with_config_name(|name| {
            self.ensure_property_not_final(name)?;
            let property = self
                .properties
                .entry(name.to_string())
                .or_insert_with(|| Property::new(name));

            property.set(values).map_err(ConfigError::from)
        })
    }

    /// Adds configuration values
    ///
    /// Adds values to an existing configuration item (multi-value properties).
    ///
    /// # Type Parameters
    ///
    /// * `T` - Element type, automatically inferred from the `values` parameter
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
    pub fn add<S>(&mut self, name: impl ConfigName, values: S) -> ConfigResult<()>
    where
        S: for<'a> MultiValuesAddArg<'a, Item = <S as MultiValuesSetArg<'static>>::Item>
            + for<'a> MultiValuesSetArg<'a>,
        <S as MultiValuesSetArg<'static>>::Item: Clone,
        MultiValues: MultiValuesAdder<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesMultiAdder<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetter<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetterSlice<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSingleSetter<<S as MultiValuesSetArg<'static>>::Item>,
    {
        name.with_config_name(|name| {
            self.ensure_property_not_final(name)?;

            if let Some(property) = self.properties.get_mut(name) {
                property.add(values).map_err(ConfigError::from)
            } else {
                let mut property = Property::new(name);
                property.set(values).map_err(ConfigError::from)?;
                self.properties.insert(name.to_string(), property);
                Ok(())
            }
        })
    }

    // ========================================================================
    // String Special Handling (Variable Substitution)
    // ========================================================================

    /// Gets a string configuration value (with variable substitution)
    ///
    /// If variable substitution is enabled, replaces `${var_name}` placeholders
    /// in the stored string.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns the string value on success, or an error on failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("base_url", "http://localhost").unwrap();
    /// config.set("api_url", "${base_url}/api").unwrap();
    ///
    /// let api_url = config.get_string("api_url").unwrap();
    /// assert_eq!(api_url, "http://localhost/api");
    /// ```
    pub fn get_string(&self, name: impl ConfigName) -> ConfigResult<String> {
        <Self as ConfigReader>::get_string(self, name)
    }

    /// Gets a string value from the first present and non-empty key in `names`.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// Returns the string value on success, or an error on failure.
    pub fn get_string_any(&self, names: impl ConfigNames) -> ConfigResult<String> {
        <Self as ConfigReader>::get_string_any(self, names)
    }

    /// Gets an optional string value from the first present and non-empty key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` when all keys are missing or empty.
    pub fn get_optional_string_any(&self, names: impl ConfigNames) -> ConfigResult<Option<String>> {
        <Self as ConfigReader>::get_optional_string_any(self, names)
    }

    /// Gets a string from any key, or `default` when all keys are missing or
    /// empty.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys checked in priority order.
    /// * `default` - Fallback used only when all keys are missing or empty.
    ///
    /// # Returns
    ///
    /// Returns the string value or default value. Substitution errors are
    /// returned instead of being hidden by the default.
    pub fn get_string_any_or(
        &self,
        names: impl ConfigNames,
        default: &str,
    ) -> ConfigResult<String> {
        <Self as ConfigReader>::get_string_any_or(self, names, default)
    }

    /// Gets a string with substitution, or `default` if the key is absent or
    /// empty.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value
    ///
    /// # Returns
    ///
    /// Returns the string value or default value. Substitution errors are
    /// returned instead of being hidden by the default.
    ///
    pub fn get_string_or(&self, name: impl ConfigName, default: &str) -> ConfigResult<String> {
        <Self as ConfigReader>::get_string_or(self, name, default)
    }

    /// Gets a list of string configuration values (with variable substitution)
    ///
    /// If variable substitution is enabled, runs it on each list element
    /// (same `${var_name}` rules as [`Self::get_string`]).
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns a list of strings on success, or an error on failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("base_path", "/opt/app").unwrap();
    /// config.set("paths", vec!["${base_path}/bin", "${base_path}/lib"]).unwrap();
    ///
    /// let paths = config.get_string_list("paths").unwrap();
    /// assert_eq!(paths, vec!["/opt/app/bin", "/opt/app/lib"]);
    /// ```
    pub fn get_string_list(&self, name: impl ConfigName) -> ConfigResult<Vec<String>> {
        <Self as ConfigReader>::get_string_list(self, name)
    }

    /// Gets a list of string configuration values or returns a default value
    /// (with variable substitution)
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value (can be array slice or vec)
    ///
    /// # Returns
    ///
    /// Returns the list of strings or default value. Substitution and parsing
    /// errors are returned instead of being hidden by the default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::new();
    ///
    /// // Using array slice
    /// let paths = config.get_string_list_or("paths", &["/default/path"]).unwrap();
    /// assert_eq!(paths, vec!["/default/path"]);
    ///
    /// // Using vec
    /// let paths = config.get_string_list_or("paths", &vec!["path1", "path2"]).unwrap();
    /// assert_eq!(paths, vec!["path1", "path2"]);
    /// ```
    pub fn get_string_list_or(
        &self,
        name: impl ConfigName,
        default: &[&str],
    ) -> ConfigResult<Vec<String>> {
        <Self as ConfigReader>::get_string_list_or(self, name, default)
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
    /// Only variables starting with `prefix` are loaded. The prefix is stripped,
    /// the remaining key is lowercased, and underscores are converted to dots.
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
    #[inline]
    pub fn from_env_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let source = EnvFileConfigSource::from_file(path);
        Self::from_source(&source)
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
    /// ```
    #[inline]
    pub fn merge_from_source(&mut self, source: &dyn ConfigSource) -> ConfigResult<()> {
        let mut staged = self.clone();
        source.load(&mut staged)?;
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
    /// assert!(config.contains_prefix("http."));
    /// assert!(!config.contains_prefix("db."));
    /// ```
    #[inline]
    pub fn contains_prefix(&self, prefix: &str) -> bool {
        self.properties.keys().any(|k| k.starts_with(prefix))
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
    /// assert!(http_config.contains("host"));
    /// assert!(http_config.contains("port"));
    /// assert!(!http_config.contains("db.host"));
    /// ```
    pub fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config> {
        let mut sub = Config::new();
        sub.description = self.description.clone();
        sub.enable_variable_substitution = self.enable_variable_substitution;
        sub.max_substitution_depth = self.max_substitution_depth;
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
                sub.properties.insert(new_key, v.clone());
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
    /// `true` if the property exists and has no values (is empty).
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
    /// assert!(config.is_null("nullable"));
    /// assert!(!config.is_null("missing"));
    /// ```
    pub fn is_null(&self, name: impl ConfigName) -> bool {
        name.with_config_name(|name| {
            self.properties
                .get(name)
                .map(|p| p.is_empty())
                .unwrap_or(false)
        })
    }

    /// Gets an optional configuration value.
    ///
    /// Distinguishes between three states:
    /// - `Ok(Some(value))` – key exists and has a value
    /// - `Ok(None)` – key does not exist, **or** exists but is null/empty
    /// - `Err(e)` – key exists and has a value, but conversion failed
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
    pub fn get_optional<T>(&self, name: impl ConfigName) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional(self, name)
    }

    /// Gets an optional list of configuration values.
    ///
    /// See also [`Self::get_optional_string_list`] for optional string lists
    /// with variable substitution.
    ///
    /// Distinguishes between three states:
    /// - `Ok(Some(vec))` – key exists and has values
    /// - `Ok(None)` – key does not exist, **or** exists but is null/empty
    /// - `Err(e)` – key exists and has values, but conversion failed
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target element type supported by [`FromConfig`]
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, `Ok(None)`, or `Err` as described above.
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
    pub fn get_optional_list<T>(&self, name: impl ConfigName) -> ConfigResult<Option<Vec<T>>>
    where
        T: FromConfig,
    {
        <Self as ConfigReader>::get_optional(self, name)
    }

    /// Gets an optional string (with variable substitution when enabled).
    ///
    /// Same semantics as [`Self::get_optional`], but values are read via
    /// [`Self::get_string`], so `${...}` substitution applies when enabled.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `Ok(Some(s))`, `Ok(None)`, or `Err` as for [`Self::get_optional`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("base", "http://localhost").unwrap();
    /// config.set("api", "${base}/api").unwrap();
    ///
    /// let api = config.get_optional_string("api").unwrap();
    /// assert_eq!(api.as_deref(), Some("http://localhost/api"));
    ///
    /// let missing = config.get_optional_string("missing").unwrap();
    /// assert_eq!(missing, None);
    /// ```
    pub fn get_optional_string(&self, name: impl ConfigName) -> ConfigResult<Option<String>> {
        <Self as ConfigReader>::get_optional_string(self, name)
    }

    /// Gets an optional string list (substitution per element when enabled).
    ///
    /// Same semantics as [`Self::get_optional_list`], but elements use
    /// [`Self::get_string_list`] (same `${...}` rules as [`Self::get_string`]).
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, `Ok(None)`, or `Err` like [`Self::get_optional_list`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("root", "/opt/app").unwrap();
    /// config.set("paths", vec!["${root}/bin", "${root}/lib"]).unwrap();
    ///
    /// let paths = config.get_optional_string_list("paths").unwrap();
    /// assert_eq!(
    ///     paths,
    ///     Some(vec![
    ///         "/opt/app/bin".to_string(),
    ///         "/opt/app/lib".to_string(),
    ///     ]),
    /// );
    /// ```
    pub fn get_optional_string_list(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<Vec<String>>> {
        <Self as ConfigReader>::get_optional_string_list(self, name)
    }

    // ========================================================================
    // Structured Config Deserialization (v0.4.0)
    // ========================================================================

    /// Deserializes a config value or subtree at `prefix` into `T` using `serde`.
    /// String values inside the generated serde value apply the same
    /// `${...}` substitution rules as [`Self::get_string`] and
    /// [`Self::get_string_list`] when substitution is enabled. Scalar strings
    /// are then parsed with this config's [`ConfigReadOptions`], so
    /// environment-style booleans, numeric strings, and scalar string lists
    /// behave consistently with typed `get` reads.
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
    /// `http.*` is a [`ConfigError::KeyConflict`], as are ambiguous dotted paths
    /// such as `a` and `a.b` inside the same deserialized object.
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
    /// Returns [`ConfigError::KeyConflict`] for ambiguous key shapes,
    /// substitution/conversion errors while preparing string values, or
    /// [`ConfigError::DeserializeError`] when serde cannot deserialize the
    /// prepared value into `T`.
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
        let value = self.deserialize_root_value(prefix)?;

        match T::deserialize(ConfigValueDeserializer::new(
            value,
            prefix.to_string(),
            self.read_options(),
        )) {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_config_error(prefix)),
        }
    }

    /// Builds the JSON root consumed by structured serde deserialization.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::KeyConflict`] when `prefix` has both an exact value
    /// and child keys, or when dotted child keys cannot form an unambiguous object
    /// tree. Returns substitution/conversion errors if configured string handling
    /// fails before deserialization starts.
    fn deserialize_root_value(&self, prefix: &str) -> ConfigResult<JsonValue> {
        if prefix.is_empty() {
            return self.deserialize_subtree_value(prefix);
        }

        let exact = self.properties.get(prefix);
        let has_children = self.properties.keys().any(|key| is_child_key(key, prefix));
        match (exact, has_children) {
            (Some(_), true) => Err(ConfigError::KeyConflict {
                path: prefix.to_string(),
                existing: "exact value".to_string(),
                incoming: "nested child keys".to_string(),
            }),
            (Some(property), false) => self.deserialize_exact_value(prefix, property),
            (None, _) => self.deserialize_subtree_value(prefix),
        }
    }

    /// Builds a JSON value from a single exact property for deserialization.
    ///
    /// # Errors
    ///
    /// Returns substitution errors when string leaves contain unresolved
    /// placeholders, or `JsonValue::Null` when the exact property is effectively
    /// missing under the active read options.
    fn deserialize_exact_value(&self, key: &str, property: &Property) -> ConfigResult<JsonValue> {
        if scalar_string_is_missing_for_deserialize(self, self, key, property, self.read_options())?
        {
            return Ok(JsonValue::Null);
        }

        let mut value = utils::property_to_json_value(property);
        utils::substitute_json_strings_with_fallback(&mut value, self, self)?;
        Ok(value)
    }

    /// Builds a JSON object from keys under `prefix` for deserialization.
    ///
    /// # Errors
    ///
    /// Returns key-conflict errors for ambiguous dotted paths, and propagates
    /// substitution/conversion errors from active read options.
    fn deserialize_subtree_value(&self, prefix: &str) -> ConfigResult<JsonValue> {
        let sub = self.subconfig(prefix, true)?;

        let mut properties = sub.properties.iter().collect::<Vec<_>>();
        properties.sort_by_key(|(left_key, _)| *left_key);

        let mut map = Map::new();
        for (key, prop) in properties {
            if scalar_string_is_missing_for_deserialize(&sub, self, key, prop, self.read_options())?
            {
                continue;
            }

            let mut json_val = utils::property_to_json_value(prop);
            utils::substitute_json_strings_with_fallback(&mut json_val, &sub, self)?;
            utils::insert_deserialize_value(&mut map, key, json_val)?;
        }
        Ok(JsonValue::Object(map))
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
    pub fn set_null(&mut self, name: impl ConfigName, data_type: DataType) -> ConfigResult<()> {
        name.with_config_name(|name| {
            self.insert_property(
                name,
                Property::with_value(name, MultiValues::Empty(data_type)),
            )
        })
    }
}

impl ConfigReader for Config {
    #[inline]
    fn is_enable_variable_substitution(&self) -> bool {
        Config::is_enable_variable_substitution(self)
    }

    #[inline]
    fn max_substitution_depth(&self) -> usize {
        Config::max_substitution_depth(self)
    }

    #[inline]
    fn read_options(&self) -> &ConfigReadOptions {
        Config::read_options(self)
    }

    #[inline]
    fn description(&self) -> Option<&str> {
        Config::description(self)
    }

    #[inline]
    fn get_property(&self, name: impl ConfigName) -> Option<&Property> {
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
    fn contains(&self, name: impl ConfigName) -> bool {
        Config::contains(self, name)
    }

    #[inline]
    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        Config::get_strict(self, name)
    }

    #[inline]
    fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: FromConfig,
    {
        Config::get_list(self, name)
    }

    #[inline]
    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        Config::get_list_strict(self, name)
    }

    #[inline]
    fn get_optional_list<T>(&self, name: impl ConfigName) -> ConfigResult<Option<Vec<T>>>
    where
        T: FromConfig,
    {
        Config::get_optional_list(self, name)
    }

    #[inline]
    fn contains_prefix(&self, prefix: &str) -> bool {
        Config::contains_prefix(self, prefix)
    }

    #[inline]
    fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a> {
        Box::new(Config::iter_prefix(self, prefix))
    }

    #[inline]
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a> {
        Box::new(Config::iter(self))
    }

    #[inline]
    fn is_null(&self, name: impl ConfigName) -> bool {
        Config::is_null(self, name)
    }

    #[inline]
    fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config> {
        Config::subconfig(self, prefix, strip_prefix)
    }

    #[inline]
    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        Config::deserialize(self, prefix)
    }

    #[inline]
    fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'_> {
        Config::prefix_view(self, prefix)
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
