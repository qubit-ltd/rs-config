// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Configuration metadata and property access operations.

use qubit_utils::Transient;

use super::Config;
use crate::ConfigError;
use crate::ConfigName;
use crate::ConfigResult;
use crate::Property;
use crate::config_path::ensure_config_key;
use crate::config_property_mut::ConfigPropertyMut;
use crate::config_reader::ConfigReader;
use crate::config_section::ConfigSection;
use crate::options::ReadPolicy;

impl Config {
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

    /// Gets the default runtime read policy.
    ///
    /// # Returns
    ///
    /// The policy used by direct reads such as `get` and `get_any`.
    #[inline(always)]
    pub fn default_read_policy(&self) -> &ReadPolicy {
        self.default_read_policy.get()
    }

    /// Sets the default runtime read policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - New default read policy.
    ///
    /// # Returns
    ///
    /// Mutable reference to this configuration for chaining.
    #[inline(always)]
    pub fn set_default_read_policy(&mut self, policy: ReadPolicy) -> &mut Self {
        self.default_read_policy = Transient::new(policy);
        self
    }

    /// Creates a read-only root view using `policy` without changing this
    /// configuration's default policy.
    #[inline]
    pub fn read_with<'a>(&'a self, policy: &'a ReadPolicy) -> ConfigSection<'a> {
        <Self as ConfigReader>::read_with(self, policy)
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

    /// Creates a read-only section when it has visible descendant properties.
    ///
    /// An exact scalar at `path` does not make the section present. The
    /// returned section uses strict relative keys and this configuration's
    /// default read policy.
    ///
    /// # Arguments
    ///
    /// * `path` - Canonical section path; an empty path checks the root.
    ///
    /// # Returns
    ///
    /// `Some` with the section when descendants exist, otherwise `None`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidPath`] when `path` is not canonical.
    #[inline(always)]
    pub fn section_if_present(&self, path: &str) -> ConfigResult<Option<ConfigSection<'_>>> {
        <Self as ConfigReader>::section_if_present(self, path)
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
    pub fn get_property(&self, name: impl ConfigName) -> ConfigResult<Option<&Property>> {
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
    pub fn set_final(&mut self, name: impl ConfigName, is_final: bool) -> ConfigResult<()> {
        name.with_config_name(|name| {
            ensure_config_key(name)?;
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
    /// assert!(!config.contains("port").unwrap());
    /// ```
    #[inline]
    pub fn remove(&mut self, name: impl ConfigName) -> ConfigResult<Option<Property>> {
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
}
