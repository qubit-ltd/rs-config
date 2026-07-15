// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![allow(private_bounds)]

use std::borrow::Cow;

use qubit_datatype::{
    DataConvertTo,
    DataConverter,
    DataTypeOf,
};
use qubit_value::{
    MultiValues,
    Value as QubitValue,
    ValueError,
};

use crate::config::Config;
use crate::config_reader::ConfigReader;
use crate::options::ConfigReadOptions;
use crate::{
    ConfigError,
    ConfigName,
    ConfigResult,
    Property,
};

/// Read-only view of the descendants under a configuration path.
///
/// Every property name is resolved strictly relative to [`Self::path`]. An
/// exact scalar stored at the section path is not part of the section; only
/// descendants beginning with `{path}.` are visible.
#[derive(Debug, Clone)]
pub struct ConfigSection<'a> {
    /// Root configuration borrowed by this section.
    config: &'a Config,
    /// Normalized root-relative path of this section.
    path: String,
    /// Prefix used to select and strip visible descendant keys.
    child_prefix: Option<String>,
}

impl<'a> ConfigSection<'a> {
    /// Creates a nested section below this section.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative child path. Leading and trailing `.` separators are
    ///   removed; an empty path keeps the current section.
    ///
    /// # Returns
    ///
    /// A section borrowing the same root configuration.
    #[inline]
    pub fn section(&self, path: &str) -> ConfigSection<'a> {
        let child = path.trim_matches('.');
        if self.path.is_empty() {
            ConfigSection::new(self.config, child)
        } else if child.is_empty() {
            ConfigSection::new(self.config, self.path.as_str())
        } else {
            ConfigSection::new(self.config, &format!("{}.{}", self.path, child))
        }
    }

    /// Creates a section over `config` at `path`.
    ///
    /// # Arguments
    ///
    /// * `config` - Root configuration borrowed by the section.
    /// * `path` - Root-relative section path. Leading and trailing `.`
    ///   separators are removed; an empty path represents the root.
    ///
    /// # Returns
    ///
    /// A newly created configuration section.
    #[inline]
    pub(crate) fn new(config: &'a Config, path: &str) -> Self {
        let path = path.trim_matches('.').to_string();
        let child_prefix = if path.is_empty() {
            None
        } else {
            Some(format!("{path}."))
        };
        Self {
            config,
            path,
            child_prefix,
        }
    }

    /// Returns this section's normalized root-relative path.
    ///
    /// # Returns
    ///
    /// The empty string for the root section, or a path without leading or
    /// trailing `.` separators.
    #[inline(always)]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Resolves a relative property name to its root configuration key.
    ///
    /// # Arguments
    ///
    /// * `name` - Property name interpreted strictly relative to this section.
    ///
    /// # Returns
    ///
    /// A borrowed key for root or empty-name resolution, or an owned joined
    /// key for a non-root section property.
    #[inline]
    fn resolve_key_cow<'b>(&'b self, name: &'b str) -> Cow<'b, str> {
        if self.path.is_empty() {
            Cow::Borrowed(name)
        } else if name.is_empty() {
            Cow::Borrowed(self.path.as_str())
        } else {
            Cow::Owned(format!("{}.{}", self.path, name))
        }
    }

    /// Resolves a visible property name to its root configuration key.
    ///
    /// # Arguments
    ///
    /// * `name` - Relative property name.
    ///
    /// # Returns
    ///
    /// `None` for an empty property name on a non-root section; otherwise the
    /// resolved key.
    #[inline]
    fn visible_property_key<'b>(
        &'b self,
        name: &'b str,
    ) -> Option<Cow<'b, str>> {
        if !self.path.is_empty() && name.is_empty() {
            None
        } else {
            Some(self.resolve_key_cow(name))
        }
    }

    /// Iterates entries visible as descendants of this section.
    ///
    /// # Returns
    ///
    /// Root entries unchanged for the root section, or descendant entries with
    /// the section prefix stripped for a non-root section.
    fn visible_entries<'b>(
        &'b self,
    ) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        let Some(child_prefix) = self.child_prefix.as_deref() else {
            return Box::new(
                self.config
                    .properties
                    .iter()
                    .map(|(key, value)| (key.as_str(), value)),
            );
        };
        Box::new(self.config.properties.iter().filter_map(
            move |(key, value)| {
                key.strip_prefix(child_prefix)
                    .map(|relative| (relative, value))
            },
        ))
    }

    /// Resolves a relative subtree path against this section.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative subtree path. Leading and trailing `.` separators
    ///   are removed.
    ///
    /// # Returns
    ///
    /// The canonical root-relative subtree path.
    fn effective_root_path(&self, path: &str) -> String {
        let child = path.trim_matches('.');
        if self.path.is_empty() {
            child.to_string()
        } else if child.is_empty() {
            self.path.clone()
        } else {
            format!("{}.{}", self.path, child)
        }
    }

    /// Creates a missing-property error for an invisible relative name.
    ///
    /// # Arguments
    ///
    /// * `name` - Relative property name.
    ///
    /// # Returns
    ///
    /// A root-relative [`ConfigError::PropertyNotFound`] value.
    #[inline]
    fn missing_property_error(&self, name: &str) -> ConfigError {
        ConfigError::PropertyNotFound(self.resolve_key_cow(name).into_owned())
    }
}

impl<'a> ConfigReader for ConfigSection<'a> {
    #[inline(always)]
    fn is_enable_variable_substitution(&self) -> bool {
        self.config.is_enable_variable_substitution()
    }

    #[inline(always)]
    fn max_substitution_depth(&self) -> usize {
        self.config.max_substitution_depth()
    }

    #[inline(always)]
    fn read_options(&self) -> &ConfigReadOptions {
        self.config.read_options()
    }

    #[inline(always)]
    fn description(&self) -> Option<&str> {
        self.config.description()
    }

    fn get_property(&self, name: impl ConfigName) -> Option<&Property> {
        name.with_config_name(|name| {
            let key = self.visible_property_key(name)?;
            self.config.get_property(key.as_ref())
        })
    }

    fn len(&self) -> usize {
        self.visible_entries().count()
    }

    fn is_empty(&self) -> bool {
        self.visible_entries().next().is_none()
    }

    fn keys(&self) -> Vec<String> {
        self.visible_entries()
            .map(|(key, _)| key.to_string())
            .collect()
    }

    fn contains(&self, name: impl ConfigName) -> bool {
        name.with_config_name(|name| {
            self.visible_property_key(name)
                .is_some_and(|key| self.config.contains(key.as_ref()))
        })
    }

    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        for<'b> T: TryFrom<&'b QubitValue, Error = ValueError>
            + TryFrom<&'b MultiValues, Error = ValueError>,
    {
        name.with_config_name(|name| {
            let key = self
                .visible_property_key(name)
                .ok_or_else(|| self.missing_property_error(name))?;
            self.config.get_strict(key.as_ref())
        })
    }

    fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: DataTypeOf,
        for<'b> DataConverter<'b>: DataConvertTo<T>,
    {
        name.with_config_name(|name| {
            let key = self
                .visible_property_key(name)
                .ok_or_else(|| self.missing_property_error(name))?;
            self.config.get_list(key.as_ref())
        })
    }

    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        for<'b> T: TryFrom<&'b QubitValue, Error = ValueError>,
        for<'b> Vec<T>: TryFrom<&'b MultiValues, Error = ValueError>,
    {
        name.with_config_name(|name| {
            let key = self
                .visible_property_key(name)
                .ok_or_else(|| self.missing_property_error(name))?;
            self.config.get_list_strict(key.as_ref())
        })
    }

    fn get_optional_list<T>(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<Vec<T>>>
    where
        T: DataTypeOf,
        for<'b> DataConverter<'b>: DataConvertTo<T>,
    {
        name.with_config_name(|name| {
            let Some(key) = self.visible_property_key(name) else {
                return Ok(None);
            };
            self.config.get_optional_list(key.as_ref())
        })
    }

    fn contains_prefix(&self, prefix: &str) -> bool {
        self.visible_entries()
            .any(|(key, _)| key.starts_with(prefix))
    }

    fn iter_prefix<'b>(
        &'b self,
        prefix: &'b str,
    ) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        Box::new(
            self.visible_entries()
                .filter(move |(key, _)| key.starts_with(prefix)),
        )
    }

    fn iter<'b>(
        &'b self,
    ) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        self.visible_entries()
    }

    fn is_null(&self, name: impl ConfigName) -> bool {
        name.with_config_name(|name| {
            self.visible_property_key(name)
                .is_some_and(|key| self.config.is_null(key.as_ref()))
        })
    }

    fn subconfig(
        &self,
        prefix: &str,
        strip_prefix: bool,
    ) -> ConfigResult<Config> {
        self.config
            .subconfig(&self.effective_root_path(prefix), strip_prefix)
    }

    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.config.deserialize(&self.effective_root_path(prefix))
    }

    #[inline(always)]
    fn section(&self, path: &str) -> ConfigSection<'a> {
        ConfigSection::section(self, path)
    }

    fn resolve_key(&self, name: impl ConfigName) -> String {
        name.with_config_name(|name| self.resolve_key_cow(name).into_owned())
    }
}
