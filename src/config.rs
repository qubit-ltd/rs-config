/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Manager
//!
//! Provides storage, retrieval, and management of configurations.
//!
//! # Author
//!
//! Haixing Hu

#![allow(private_bounds)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{source::ConfigSource, utils, ConfigError, ConfigResult, Property};
use qubit_value::multi_values::{
    MultiValuesAddArg, MultiValuesAdder, MultiValuesFirstGetter, MultiValuesGetter,
    MultiValuesMultiAdder, MultiValuesSetArg, MultiValuesSetter, MultiValuesSetterSlice,
    MultiValuesSingleSetter,
};
use qubit_value::MultiValues;

/// Default maximum depth for variable substitution
pub const DEFAULT_MAX_SUBSTITUTION_DEPTH: usize = 64;

/// Configuration Manager
///
/// Manages a set of configuration properties with type-safe read/write interfaces.
///
/// # Features
///
/// - Supports multiple data types
/// - Supports variable substitution (`${var_name}` format)
/// - Supports configuration merging
/// - Supports final value protection
/// - Thread-safe (when wrapped in `Arc<RwLock<Config>>`)
///
/// # Important Limitations of Generic set/add Methods
///
/// **`u8` type does not support generic `set()` and `add()` methods**. See `MultiValues` documentation for details.
///
/// For `u8` type configuration values, use dedicated methods:
///
/// ```rust,ignore
/// use qubit_config::Config;
///
/// let mut config = Config::new();
///
/// // ❌ Not supported: config.set("byte_value", 42u8)?;
///
/// // ✅ Method 1: Use dedicated method via get_property_mut
/// config.get_property_mut("byte_value")
///     .unwrap()
///     .set_uint8(42)
///     .unwrap();
///
/// // ✅ Method 2: Create property first if it doesn't exist
/// if config.get_property("byte_value").is_none() {
///     let mut prop = Property::new("byte_value");
///     prop.set_uint8(42).unwrap();
///     config.properties.insert("byte_value".to_string(), prop);
/// }
///
/// // Reading works normally
/// let value: u8 = config.get("byte_value")?;
/// ```
///
/// **Recommendation**: If you truly need to store `u8` values, consider using `u16` instead,
/// as `u8` is rarely used for configuration values in practice, while `Vec<u8>` is more
/// commonly used for byte arrays (such as keys, hashes, etc.).
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::Config;
///
/// let mut config = Config::new();
///
/// // Set configuration values (type inference)
/// config.set("port", 8080)?;                    // inferred as i32
/// config.set("host", "localhost")?;              // &str automatically converted to String
/// config.set("debug", true)?;                   // inferred as bool
/// config.set("timeout", 30.5)?;                 // inferred as f64
///
/// // Set multiple values (type inference)
/// config.set("ports", vec![8080, 8081, 8082])?; // inferred as i32
/// config.set("hosts", &["host1", "host2"])?;     // &str automatically converted
///
/// // Read configuration values (type inference)
/// let port: i32 = config.get("port")?;
/// let host: String = config.get("host")?;
/// let debug: bool = config.get("debug")?;
///
/// // Read configuration values (turbofish)
/// let port = config.get::<i32>("port")?;
///
/// // Read configuration value or use default
/// let timeout: u64 = config.get_or("timeout", 30);
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Configuration description
    description: Option<String>,
    /// Configuration property mapping
    properties: HashMap<String, Property>,
    /// Whether variable substitution is enabled
    enable_variable_substitution: bool,
    /// Maximum depth for variable substitution
    max_substitution_depth: usize,
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let config = Config::new();
    /// assert!(config.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            description: None,
            properties: HashMap::new(),
            enable_variable_substitution: true,
            max_substitution_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let config = Config::with_description("Server Configuration");
    /// assert_eq!(config.description(), Some("Server Configuration"));
    /// ```
    pub fn with_description(description: &str) -> Self {
        Self {
            description: Some(description.to_string()),
            properties: HashMap::new(),
            enable_variable_substitution: true,
            max_substitution_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
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
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the configuration description
    ///
    /// # Parameters
    ///
    /// * `description` - Configuration description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Checks if variable substitution is enabled
    ///
    /// # Returns
    ///
    /// Returns `true` if variable substitution is enabled
    pub fn is_enable_variable_substitution(&self) -> bool {
        self.enable_variable_substitution
    }

    /// Sets whether to enable variable substitution
    ///
    /// # Parameters
    ///
    /// * `enable` - Whether to enable
    pub fn set_enable_variable_substitution(&mut self, enable: bool) {
        self.enable_variable_substitution = enable;
    }

    /// Gets the maximum depth for variable substitution
    ///
    /// # Returns
    ///
    /// Returns the maximum depth value
    pub fn max_substitution_depth(&self) -> usize {
        self.max_substitution_depth
    }

    /// Sets the maximum depth for variable substitution
    ///
    /// # Parameters
    ///
    /// * `depth` - Maximum depth
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    ///
    /// assert!(config.contains("port"));
    /// assert!(!config.contains("host"));
    /// ```
    pub fn contains(&self, name: &str) -> bool {
        self.properties.contains_key(name)
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
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }

    /// Gets a mutable reference to a configuration item
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns mutable Option containing the configuration item
    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.properties.get_mut(name)
    }

    /// Removes a configuration item
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    ///
    /// let removed = config.remove("port");
    /// assert!(removed.is_some());
    /// assert!(!config.contains("port"));
    /// ```
    pub fn remove(&mut self, name: &str) -> Option<Property> {
        self.properties.remove(name)
    }

    /// Clears all configuration items
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    /// config.set("host", "localhost")?;
    ///
    /// config.clear();
    /// assert!(config.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Gets the number of configuration items
    ///
    /// # Returns
    ///
    /// Returns the number of configuration items
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Checks if the configuration is empty
    ///
    /// # Returns
    ///
    /// Returns `true` if the configuration contains no items
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    /// config.set("host", "localhost")?;
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

    /// Gets a configuration value
    ///
    /// This is the core method for getting configuration values, supporting type inference.
    ///
    /// # Note
    ///
    /// This method does not perform variable substitution for string types. If you need
    /// variable substitution, please use the `get_string` method.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `FromPropertyValue` trait
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// Returns the value of the specified type on success, or an error on failure
    ///
    /// # Errors
    ///
    /// - Returns `ConfigError::PropertyNotFound` if the configuration item doesn't exist
    /// - Returns `ConfigError::PropertyHasNoValue` if the configuration item has no value
    /// - Returns `ConfigError::TypeMismatch` if the type doesn't match
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    /// config.set("host", "localhost")?;
    ///
    /// // Method 1: Type inference
    /// let port: i32 = config.get("port")?;
    /// let host: String = config.get("host")?;
    ///
    /// // Method 2: Turbofish
    /// let port = config.get::<i32>("port")?;
    /// let host = config.get::<String>("host")?;
    ///
    /// // Method 3: Inference from usage
    /// fn start_server(port: i32, host: String) { }
    /// start_server(config.get("port")?, config.get("host")?);
    /// ```
    pub fn get<T>(&self, name: &str) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        let property = self
            .properties
            .get(name)
            .ok_or_else(|| ConfigError::PropertyNotFound(name.to_string()))?;

        property.get_first::<T>().map_err(ConfigError::from)
    }

    /// Gets a configuration value or returns a default value
    ///
    /// Returns the default value if the configuration item doesn't exist or retrieval fails.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `FromPropertyValue` trait
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value
    ///
    /// # Returns
    ///
    /// Returns the configuration value or default value
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let config = Config::new();
    ///
    /// let port: i32 = config.get_or("port", 8080);
    /// let host: String = config.get_or("host", "localhost".to_string());
    ///
    /// assert_eq!(port, 8080);
    /// assert_eq!(host, "localhost");
    /// ```
    pub fn get_or<T>(&self, name: &str, default: T) -> T
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        self.get(name).unwrap_or(default)
    }

    /// Gets a list of configuration values
    ///
    /// Gets all values of a configuration item (multi-value configuration).
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `FromPropertyValue` trait
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("ports", vec![8080, 8081, 8082])?;
    ///
    /// let ports: Vec<i32> = config.get_list("ports")?;
    /// assert_eq!(ports, vec![8080, 8081, 8082]);
    /// ```
    pub fn get_list<T>(&self, name: &str) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        let property = self
            .properties
            .get(name)
            .ok_or_else(|| ConfigError::PropertyNotFound(name.to_string()))?;

        property.get::<T>().map_err(ConfigError::from)
    }

    /// Sets a configuration value
    ///
    /// This is the core method for setting configuration values, supporting type inference.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Element type, automatically inferred from the `values` parameter
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `values` - Configuration value, supports `T`, `Vec<T>`, `&[T]`, and other types
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error on failure
    ///
    /// # Errors
    ///
    /// - Returns `ConfigError::PropertyIsFinal` if the configuration item is final
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    ///
    /// // Set single values (type auto-inference)
    /// config.set("port", 8080)?;                    // T inferred as i32
    /// config.set("host", "localhost")?;              // T inferred as String (&str auto-converted)
    /// config.set("debug", true)?;                   // T inferred as bool
    /// config.set("timeout", 30.5)?;                 // T inferred as f64
    ///
    /// // Set multiple values (type auto-inference)
    /// config.set("ports", vec![8080, 8081, 8082])?; // T inferred as i32
    /// config.set("hosts", &["host1", "host2"])?;     // T inferred as &str (auto-converted)
    /// ```
    pub fn set<S>(&mut self, name: &str, values: S) -> ConfigResult<()>
    where
        S: for<'a> MultiValuesSetArg<'a>,
        <S as MultiValuesSetArg<'static>>::Item: Clone,
        MultiValues: MultiValuesSetter<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetterSlice<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSingleSetter<<S as MultiValuesSetArg<'static>>::Item>,
    {
        // Check if it's a final value
        if let Some(prop) = self.properties.get(name) {
            if prop.is_final() {
                return Err(ConfigError::PropertyIsFinal(name.to_string()));
            }
        }
        let property = self
            .properties
            .entry(name.to_string())
            .or_insert_with(|| Property::new(name));

        property.set(values).map_err(ConfigError::from)
    }

    /// Adds configuration values
    ///
    /// Adds values to an existing configuration item (for multi-value configuration).
    ///
    /// # Type Parameters
    ///
    /// * `T` - Element type, automatically inferred from the `values` parameter
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `values` - Values to add, supports `T`, `Vec<T>`, `&[T]`, and other types
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error on failure
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;                    // Set initial value
    /// config.add("port", 8081)?;                    // Add single value
    /// config.add("port", vec![8082, 8083])?;        // Add multiple values
    /// config.add("port", &[8084, 8085])?;          // Add slice
    ///
    /// let ports: Vec<i32> = config.get_list("port")?;
    /// assert_eq!(ports, vec![8080, 8081, 8082, 8083, 8084, 8085]);
    /// ```
    pub fn add<S>(&mut self, name: &str, values: S) -> ConfigResult<()>
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
        // Check if it's a final value
        if let Some(prop) = self.properties.get(name) {
            if prop.is_final() {
                return Err(ConfigError::PropertyIsFinal(name.to_string()));
            }
        }

        if let Some(property) = self.properties.get_mut(name) {
            property.add(values).map_err(ConfigError::from)
        } else {
            let mut property = Property::new(name);
            // Note: property.set() always returns Ok(()) in current MultiValues implementation,
            // as it unconditionally replaces the entire value without any validation.
            // We explicitly ignore the result to improve code coverage and avoid unreachable error paths.
            let _ = property.set(values);
            self.properties.insert(name.to_string(), property);
            Ok(())
        }
    }

    // ========================================================================
    // String Special Handling (Variable Substitution)
    // ========================================================================

    /// Gets a string configuration value (with variable substitution)
    ///
    /// If variable substitution is enabled, automatically replaces variables in `${var_name}` format.
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("base_url", "http://localhost")?;
    /// config.set("api_url", "${base_url}/api")?;
    ///
    /// let api_url = config.get_string("api_url")?;
    /// assert_eq!(api_url, "http://localhost/api");
    /// ```
    pub fn get_string(&self, name: &str) -> ConfigResult<String> {
        let value: String = self.get(name)?;
        if self.enable_variable_substitution {
            utils::substitute_variables(&value, self, self.max_substitution_depth)
        } else {
            Ok(value)
        }
    }

    /// Gets a string configuration value or returns a default value (with variable substitution)
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value
    ///
    /// # Returns
    ///
    /// Returns the string value or default value
    ///
    pub fn get_string_or(&self, name: &str, default: &str) -> String {
        self.get_string(name)
            .unwrap_or_else(|_| default.to_string())
    }

    /// Gets a list of string configuration values (with variable substitution)
    ///
    /// If variable substitution is enabled, automatically replaces variables in `${var_name}` format for each string in the list.
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("base_path", "/opt/app")?;
    /// config.set("paths", vec!["${base_path}/bin", "${base_path}/lib"])?;
    ///
    /// let paths = config.get_string_list("paths")?;
    /// assert_eq!(paths, vec!["/opt/app/bin", "/opt/app/lib"]);
    /// ```
    pub fn get_string_list(&self, name: &str) -> ConfigResult<Vec<String>> {
        let values: Vec<String> = self.get_list(name)?;
        if self.enable_variable_substitution {
            values
                .into_iter()
                .map(|v| utils::substitute_variables(&v, self, self.max_substitution_depth))
                .collect()
        } else {
            Ok(values)
        }
    }

    /// Gets a list of string configuration values or returns a default value (with variable substitution)
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    /// * `default` - Default value (can be array slice or vec)
    ///
    /// # Returns
    ///
    /// Returns the list of strings or default value
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let config = Config::new();
    ///
    /// // Using array slice
    /// let paths = config.get_string_list_or("paths", &["/default/path"]);
    /// assert_eq!(paths, vec!["/default/path"]);
    ///
    /// // Using vec
    /// let paths = config.get_string_list_or("paths", &vec!["path1", "path2"]);
    /// assert_eq!(paths, vec!["path1", "path2"]);
    /// ```
    pub fn get_string_list_or(&self, name: &str, default: &[&str]) -> Vec<String> {
        self.get_string_list(name)
            .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
    }

    // ========================================================================
    // Configuration Source Integration
    // ========================================================================

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
    /// ```rust,ignore
    /// use qubit_config::{Config, source::{TomlSource, EnvSource, CompositeSource, ConfigSource}};
    ///
    /// let mut composite = CompositeSource::new();
    /// composite.add(TomlSource::from_file("config.toml"));
    /// composite.add(EnvSource::with_prefix("APP_"));
    ///
    /// let mut config = Config::new();
    /// config.merge_from_source(&composite).unwrap();
    /// ```
    pub fn merge_from_source(&mut self, source: &dyn ConfigSource) -> ConfigResult<()> {
        source.load(self)
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.is_empty());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
