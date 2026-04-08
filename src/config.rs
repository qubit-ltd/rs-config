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
use qubit_value::ValueError;

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

        property.get_first::<T>().map_err(|e| match e {
            ValueError::NoValue => ConfigError::PropertyHasNoValue(name.to_string()),
            ValueError::TypeMismatch { expected, actual } => {
                ConfigError::type_mismatch_at(name, expected, actual)
            }
            ValueError::ConversionFailed { from, to } => {
                ConfigError::conversion_error_at(name, format!("From {from} to {to}"))
            }
            ValueError::ConversionError(msg) => ConfigError::conversion_error_at(name, msg),
            ValueError::IndexOutOfBounds { index, len } => {
                ConfigError::IndexOutOfBounds { index, len }
            }
            ValueError::JsonSerializationError(msg) => {
                ConfigError::conversion_error_at(name, format!("JSON serialization error: {msg}"))
            }
            ValueError::JsonDeserializationError(msg) => {
                ConfigError::conversion_error_at(name, format!("JSON deserialization error: {msg}"))
            }
        })
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

        property.get::<T>().map_err(|e| match e {
            ValueError::NoValue => ConfigError::PropertyHasNoValue(name.to_string()),
            ValueError::TypeMismatch { expected, actual } => {
                ConfigError::type_mismatch_at(name, expected, actual)
            }
            ValueError::ConversionFailed { from, to } => {
                ConfigError::conversion_error_at(name, format!("From {from} to {to}"))
            }
            ValueError::ConversionError(msg) => ConfigError::conversion_error_at(name, msg),
            ValueError::IndexOutOfBounds { index, len } => {
                ConfigError::IndexOutOfBounds { index, len }
            }
            ValueError::JsonSerializationError(msg) => {
                ConfigError::conversion_error_at(name, format!("JSON serialization error: {msg}"))
            }
            ValueError::JsonDeserializationError(msg) => {
                ConfigError::conversion_error_at(name, format!("JSON deserialization error: {msg}"))
            }
        })
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
    /// use qubit_config::{Config, source::{TomlConfigSource, EnvConfigSource, CompositeConfigSource, ConfigSource}};
    ///
    /// let mut composite = CompositeConfigSource::new();
    /// composite.add(TomlConfigSource::from_file("config.toml"));
    /// composite.add(EnvConfigSource::with_prefix("APP_"));
    ///
    /// let mut config = Config::new();
    /// config.merge_from_source(&composite).unwrap();
    /// ```
    pub fn merge_from_source(&mut self, source: &dyn ConfigSource) -> ConfigResult<()> {
        source.load(self)
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("host", "localhost")?;
    /// config.set("port", 8080)?;
    ///
    /// for (key, prop) in config.iter() {
    ///     println!("{} = {:?}", key, prop);
    /// }
    /// ```
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
    /// An iterator yielding `(&str, &Property)` tuples where the key starts with `prefix`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost")?;
    /// config.set("http.port", 8080)?;
    /// config.set("db.host", "dbhost")?;
    ///
    /// let http_entries: Vec<_> = config.iter_prefix("http.").collect();
    /// assert_eq!(http_entries.len(), 2);
    /// ```
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
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost")?;
    ///
    /// assert!(config.contains_prefix("http."));
    /// assert!(!config.contains_prefix("db."));
    /// ```
    pub fn contains_prefix(&self, prefix: &str) -> bool {
        self.properties.keys().any(|k| k.starts_with(prefix))
    }

    /// Extracts a sub-configuration for keys matching `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to extract (e.g., `"http"`)
    /// * `strip_prefix` - If `true`, the prefix and the following `.` separator are stripped
    ///   from the keys in the returned `Config`. If `false`, keys are kept as-is.
    ///
    /// # Returns
    ///
    /// A new `Config` containing only the matching entries.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost")?;
    /// config.set("http.port", 8080)?;
    /// config.set("db.host", "dbhost")?;
    ///
    /// let http_config = config.subconfig("http", true)?;
    /// assert!(http_config.contains("host"));
    /// assert!(http_config.contains("port"));
    /// assert!(!http_config.contains("db.host"));
    /// ```
    pub fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config> {
        let mut sub = Config::new();
        sub.enable_variable_substitution = self.enable_variable_substitution;
        sub.max_substitution_depth = self.max_substitution_depth;

        // Empty prefix means "all keys"
        if prefix.is_empty() {
            for (k, v) in &self.properties {
                sub.properties.insert(k.clone(), v.clone());
            }
            return Ok(sub);
        }

        let full_prefix = format!("{prefix}.");

        for (k, v) in &self.properties {
            if k == prefix || k.starts_with(&full_prefix) {
                let new_key = if strip_prefix {
                    if k == prefix {
                        prefix.to_string()
                    } else {
                        k[full_prefix.len()..].to_string()
                    }
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

    /// Returns `true` if the property exists but has no value (i.e., is empty/null).
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
    /// ```rust,ignore
    /// use qubit_config::{Config, Property};
    /// use qubit_value::MultiValues;
    /// use qubit_common::DataType;
    ///
    /// let mut config = Config::new();
    /// config.properties_mut().insert(
    ///     "nullable".to_string(),
    ///     Property::with_value("nullable", MultiValues::Empty(DataType::String)),
    /// );
    ///
    /// assert!(config.is_null("nullable"));
    /// assert!(!config.is_null("missing"));
    /// ```
    pub fn is_null(&self, name: &str) -> bool {
        self.properties
            .get(name)
            .map(|p| p.is_empty())
            .unwrap_or(false)
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
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("port", 8080)?;
    ///
    /// let port: Option<i32> = config.get_optional("port")?;
    /// assert_eq!(port, Some(8080));
    ///
    /// let missing: Option<i32> = config.get_optional("missing")?;
    /// assert_eq!(missing, None);
    /// ```
    pub fn get_optional<T>(&self, name: &str) -> ConfigResult<Option<T>>
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        match self.properties.get(name) {
            None => Ok(None),
            Some(prop) if prop.is_empty() => Ok(None),
            Some(_) => self.get::<T>(name).map(Some),
        }
    }

    /// Gets an optional list of configuration values.
    ///
    /// Distinguishes between three states:
    /// - `Ok(Some(vec))` – key exists and has values
    /// - `Ok(None)` – key does not exist, **or** exists but is null/empty
    /// - `Err(e)` – key exists and has values, but conversion failed
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target element type
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("ports", vec![8080, 8081])?;
    ///
    /// let ports: Option<Vec<i32>> = config.get_list_optional("ports")?;
    /// assert_eq!(ports, Some(vec![8080, 8081]));
    ///
    /// let missing: Option<Vec<i32>> = config.get_list_optional("missing")?;
    /// assert_eq!(missing, None);
    /// ```
    pub fn get_list_optional<T>(&self, name: &str) -> ConfigResult<Option<Vec<T>>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        match self.properties.get(name) {
            None => Ok(None),
            Some(prop) if prop.is_empty() => Ok(None),
            Some(_) => self.get_list::<T>(name).map(Some),
        }
    }

    // ========================================================================
    // Structured Config Deserialization (v0.4.0)
    // ========================================================================

    /// Deserializes a sub-configuration (identified by `prefix`) into a struct `T`.
    ///
    /// The keys under `prefix` (with the prefix and its trailing `.` stripped) are
    /// presented to `serde` as a flat map, so a struct like:
    ///
    /// ```rust,ignore
    /// #[derive(serde::Deserialize)]
    /// struct HttpOptions {
    ///     host: String,
    ///     port: u16,
    /// }
    /// ```
    ///
    /// can be populated from config keys `http.host` and `http.port` by calling
    /// `config.deserialize::<HttpOptions>("http")`.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix that scopes the struct fields (use `""` for the root)
    ///
    /// # Returns
    ///
    /// Returns the deserialized value on success, or a `ConfigError` on failure.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// config.set("server.host", "localhost")?;
    /// config.set("server.port", 8080)?;
    ///
    /// let server: Server = config.deserialize("server")?;
    /// assert_eq!(server.host, "localhost");
    /// assert_eq!(server.port, 8080);
    /// ```
    pub fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use serde_json::{Map, Value as JsonValue};

        let sub = self.subconfig(prefix, true)?;

        let mut map = Map::new();
        for (key, prop) in &sub.properties {
            let json_val = property_to_json_value(prop);
            map.insert(key.clone(), json_val);
        }

        let json_obj = JsonValue::Object(map);

        serde_json::from_value(json_obj).map_err(|e| ConfigError::DeserializeError {
            path: prefix.to_string(),
            message: e.to_string(),
        })
    }

    /// Returns a mutable reference to the internal properties map.
    ///
    /// This is primarily intended for advanced use cases such as directly inserting
    /// null/empty properties that cannot be expressed via the normal `set()` API.
    pub fn properties_mut(&mut self) -> &mut HashMap<String, Property> {
        &mut self.properties
    }
}

/// Converts a `Property` to a `serde_json::Value` for use in structured deserialization.
fn property_to_json_value(prop: &Property) -> serde_json::Value {
    use qubit_value::MultiValues;
    use serde_json::Value as JsonValue;

    let mv = prop.value();

    match mv {
        MultiValues::Empty(_) => JsonValue::Null,
        MultiValues::Bool(v) => {
            if v.len() == 1 {
                JsonValue::Bool(v[0])
            } else {
                JsonValue::Array(v.iter().map(|b| JsonValue::Bool(*b)).collect())
            }
        }
        MultiValues::Int8(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::Int16(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::Int32(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::Int64(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::IntSize(v) => scalar_or_array(v, |x| {
            JsonValue::Number(serde_json::Number::from(*x as i64))
        }),
        MultiValues::UInt8(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::UInt16(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::UInt32(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::UInt64(v) => scalar_or_array(v, |x| JsonValue::Number((*x).into())),
        MultiValues::UIntSize(v) => scalar_or_array(v, |x| {
            JsonValue::Number(serde_json::Number::from(*x as u64))
        }),
        MultiValues::Float32(v) => scalar_or_array(v, |x| {
            serde_json::Number::from_f64(*x as f64)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }),
        MultiValues::Float64(v) => scalar_or_array(v, |x| {
            serde_json::Number::from_f64(*x)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }),
        MultiValues::String(v) => scalar_or_array(v, |x| JsonValue::String(x.clone())),
        MultiValues::Duration(v) => {
            scalar_or_array(v, |x| JsonValue::String(format!("{}ms", x.as_millis())))
        }
        MultiValues::Url(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::StringMap(v) => {
            if v.len() == 1 {
                let obj: serde_json::Map<String, JsonValue> = v[0]
                    .iter()
                    .map(|(k, val)| (k.clone(), JsonValue::String(val.clone())))
                    .collect();
                JsonValue::Object(obj)
            } else {
                JsonValue::Array(
                    v.iter()
                        .map(|m| {
                            let obj: serde_json::Map<String, JsonValue> = m
                                .iter()
                                .map(|(k, val)| (k.clone(), JsonValue::String(val.clone())))
                                .collect();
                            JsonValue::Object(obj)
                        })
                        .collect(),
                )
            }
        }
        MultiValues::Json(v) => {
            if v.len() == 1 {
                v[0].clone()
            } else {
                JsonValue::Array(v.clone())
            }
        }
        MultiValues::Char(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::BigInteger(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::BigDecimal(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::DateTime(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::Date(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::Time(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::Instant(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::Int128(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
        MultiValues::UInt128(v) => scalar_or_array(v, |x| JsonValue::String(x.to_string())),
    }
}

/// Helper: if the vec has exactly one element, return a scalar JSON value; otherwise an array.
fn scalar_or_array<T, F>(v: &[T], f: F) -> serde_json::Value
where
    F: Fn(&T) -> serde_json::Value,
{
    if v.len() == 1 {
        f(&v[0])
    } else {
        serde_json::Value::Array(v.iter().map(f).collect())
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
