// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Typed and structured configuration reads.

use super::Config;
use crate::config_reader::ConfigReader;
use crate::from::{
    FromConfig,
    IntoConfigDefault,
};
use crate::options::ReadPolicy;
use crate::utils;
use crate::{
    ConfigName,
    ConfigNames,
    ConfigResult,
    ConfigSection,
    Property,
};
use qubit_datatype::DataConversionTarget;
use qubit_value::{
    StrictValueListRead,
    StrictValueRead,
};

impl Config {
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

    // ========================================================================
    // Optional and Unset Semantics
    // ========================================================================

    /// Returns `true` if the property exists and is unset.
    ///
    /// This distinguishes between:
    /// - Key does not exist → `contains()` returns `false` and this returns
    ///   `false`.
    /// - Key exists with an unset value → this returns `true`.
    /// - Key exists with a concrete empty collection or empty string → this
    ///   returns `false`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name
    ///
    /// # Returns
    ///
    /// `true` only when the property exists and its value container is unset.
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
    /// assert!(config.is_unset("nullable").unwrap());
    /// assert!(!config.is_unset("missing").unwrap());
    /// ```
    pub fn is_unset(&self, name: impl ConfigName) -> ConfigResult<bool> {
        <Self as ConfigReader>::is_unset(self, name)
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
}
impl ConfigReader for Config {
    #[inline]
    fn read_policy(&self) -> &ReadPolicy {
        Config::default_read_policy(self)
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
    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: StrictValueListRead,
    {
        Config::get_list_strict(self, name)
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
    fn section(&self, path: &str) -> ConfigResult<ConfigSection<'_>> {
        Config::section(self, path)
    }
}
