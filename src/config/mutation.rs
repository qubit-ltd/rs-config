// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Configuration mutation operations.

use qubit_datatype::DataType;
use qubit_value::ValueContainer;

use super::Config;
use crate::ConfigError;
use crate::ConfigName;
use crate::ConfigResult;
use crate::Property;
use crate::config_path::ensure_config_key;
use crate::source::ConfigSource;

impl Config {
    /// Sets a configuration value.
    ///
    /// This is the core mutation operation and accepts scalar or collection
    /// values through type inference.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration item name.
    /// * `values` - Scalar or collection accepted by [`ValueContainer`].
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] when the property is final.
    pub fn set<S>(&mut self, name: impl ConfigName, values: S) -> ConfigResult<()>
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
    pub fn add<S>(&mut self, name: impl ConfigName, values: S) -> ConfigResult<()>
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

    /// Merges only properties loaded from a `ConfigSource`.
    ///
    /// The source's description and default read policy are intentionally
    /// ignored. Existing non-final properties are overwritten; final
    /// properties are preserved and cause an error if the source tries to
    /// overwrite them.
    ///
    /// # Parameters
    ///
    /// * `source` - The configuration source to load from
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. If loading or merging fails, the target
    /// configuration remains unchanged and the relevant `ConfigError` is
    /// returned.
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
    /// composite.add(EnvConfigSource::from_prefix("APP_"));
    ///
    /// let mut config = Config::new();
    /// config.merge_properties_from_source(&composite).unwrap();
    /// std::fs::remove_file(&path).unwrap();
    /// # }
    /// ```
    #[inline]
    pub fn merge_properties_from_source(&mut self, source: &dyn ConfigSource) -> ConfigResult<()> {
        let layer = source.load()?;
        self.merge_properties(layer)
    }

    /// Merges only the properties from a source-produced configuration.
    ///
    /// # Parameters
    ///
    /// * `layer` - Config layer produced by a source load.
    ///
    /// # Returns
    ///
    /// `Ok(())` when all properties are merged successfully.
    #[inline]
    pub fn merge_properties(&mut self, source_config: Config) -> ConfigResult<()> {
        // Validate every incoming property before mutating `self`. This keeps
        // the merge transactional without cloning the complete configuration.
        for (name, property) in &source_config.properties {
            ensure_config_key(name)?;
            if property.name() != name {
                return Err(ConfigError::MergeError(format!(
                    "Property name mismatch: key '{name}' != property '{}'",
                    property.name()
                )));
            }
            self.ensure_property_not_final(name)?;
        }

        let Config { mut properties, .. } = source_config;
        self.properties.append(&mut properties);
        Ok(())
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
    pub fn set_null(&mut self, name: impl ConfigName, data_type: DataType) -> ConfigResult<()> {
        name.with_config_name(|name| {
            let property = Property::new(name, ValueContainer::new_unset_scalar(data_type))?;
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
    pub(super) fn get_property_by_name(&self, name: &str) -> ConfigResult<&Property> {
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
    pub(super) fn ensure_property_not_final(&self, name: &str) -> ConfigResult<()> {
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
    pub(super) fn ensure_no_final_properties(&self) -> ConfigResult<()> {
        if let Some((name, _)) = self.properties.iter().find(|(_, prop)| prop.is_final()) {
            return Err(ConfigError::PropertyIsFinal(name.clone()));
        }
        Ok(())
    }
}
