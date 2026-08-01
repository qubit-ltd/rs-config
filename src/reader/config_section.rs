// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![allow(private_bounds)]

use std::borrow::Cow;

use qubit_value::{
    StrictValueListRead,
    StrictValueRead,
};

use crate::config::Config;
use crate::config_path::{
    ensure_config_key,
    ensure_config_path,
};
use crate::config_reader::ConfigReader;
use crate::options::ReadPolicy;
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
#[must_use]
#[derive(Debug, Clone)]
pub struct ConfigSection<'a> {
    /// Root configuration borrowed by this section.
    config: &'a Config,
    /// Normalized root-relative path of this section.
    path: String,
    /// Prefix used to select and strip visible descendant keys.
    child_prefix: Option<String>,
    /// Read policy borrowed by this view, when explicitly overridden.
    read_policy: Option<&'a ReadPolicy>,
}

impl<'a> ConfigSection<'a> {
    /// Creates a nested section below this section.
    ///
    /// # Arguments
    ///
    /// * `path` - Canonical relative child path. An empty path keeps the
    ///   current section.
    ///
    /// # Returns
    ///
    /// A section borrowing the same root configuration.
    #[inline]
    pub fn section(&self, path: &str) -> ConfigResult<ConfigSection<'a>> {
        ensure_config_path(path)?;
        if self.path.is_empty() {
            ConfigSection::new_with_read_policy(
                self.config,
                path,
                self.read_policy,
            )
        } else if path.is_empty() {
            ConfigSection::new_with_read_policy(
                self.config,
                self.path.as_str(),
                self.read_policy,
            )
        } else {
            ConfigSection::new_with_read_policy(
                self.config,
                &format!("{}.{}", self.path, path),
                self.read_policy,
            )
        }
    }

    /// Creates a section over `config` at `path`.
    ///
    /// # Arguments
    ///
    /// * `config` - Root configuration borrowed by the section.
    /// * `path` - Canonical root-relative section path. An empty path
    ///   represents the root.
    ///
    /// # Returns
    ///
    /// A newly created configuration section.
    #[inline]
    pub(crate) fn new(config: &'a Config, path: &str) -> ConfigResult<Self> {
        Self::new_with_read_policy(config, path, None)
    }

    /// Creates a section with an optional borrowed read-policy override.
    #[inline]
    fn new_with_read_policy(
        config: &'a Config,
        path: &str,
        read_policy: Option<&'a ReadPolicy>,
    ) -> ConfigResult<Self> {
        ensure_config_path(path)?;
        let path = path.to_string();
        let child_prefix = if path.is_empty() {
            None
        } else {
            Some(format!("{path}."))
        };
        Ok(Self {
            config,
            path,
            child_prefix,
            read_policy,
        })
    }

    /// Applies a borrowed read-policy override to this view.
    #[inline(always)]
    pub(crate) fn with_read_policy_override(
        mut self,
        policy: &'a ReadPolicy,
    ) -> Self {
        self.read_policy = Some(policy);
        self
    }

    /// Returns the root configuration backing this section.
    #[inline(always)]
    pub(crate) fn root_config(&self) -> &'a Config {
        self.config
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

    /// Returns this section with a borrowed read-policy override.
    ///
    /// Nested sections inherit the same override.
    ///
    /// # Parameters
    ///
    /// * `policy` - Read policy borrowed by the returned section.
    ///
    /// # Returns
    ///
    /// This scoped section with the override applied.
    #[inline(always)]
    pub fn read_with<'b>(self, policy: &'b ReadPolicy) -> ConfigSection<'b>
    where
        'a: 'b,
    {
        ConfigSection {
            config: self.config,
            path: self.path,
            child_prefix: self.child_prefix,
            read_policy: Some(policy),
        }
    }

    /// Returns whether a visible key starts with the raw character prefix.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Prefix interpreted relative to this section.
    ///
    /// # Returns
    ///
    /// `true` when a visible key starts with `prefix`.
    #[inline(always)]
    pub fn contains_key_prefix(&self, prefix: &str) -> bool {
        <Self as ConfigReader>::contains_key_prefix(self, prefix)
    }

    /// Returns whether a relative dotted section has descendants.
    ///
    /// # Parameters
    ///
    /// * `path` - Dotted path relative to this section.
    ///
    /// # Returns
    ///
    /// `true` when at least one descendant belongs to the exact section.
    #[inline(always)]
    pub fn contains_section(&self, path: &str) -> ConfigResult<bool> {
        <Self as ConfigReader>::contains_section(self, path)
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
    ) -> ConfigResult<Option<Cow<'b, str>>> {
        ensure_config_key(name)?;
        if !self.path.is_empty() && name.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.resolve_key_cow(name)))
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
    ) -> impl Iterator<Item = (&'b str, &'b Property)> + 'b {
        let child_prefix = self.child_prefix.as_deref();
        self.config
            .properties
            .iter()
            .filter_map(move |(key, value)| match child_prefix {
                Some(prefix) => {
                    key.strip_prefix(prefix).map(|relative| (relative, value))
                }
                None => Some((key.as_str(), value)),
            })
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
    fn read_policy(&self) -> &ReadPolicy {
        self.read_policy
            .unwrap_or_else(|| self.config.default_read_policy())
    }

    #[inline(always)]
    fn scope_path(&self) -> &str {
        &self.path
    }

    fn get_property(
        &self,
        name: impl ConfigName,
    ) -> ConfigResult<Option<&Property>> {
        name.with_config_name(|name| {
            let Some(key) = self.visible_property_key(name)? else {
                return Ok(None);
            };
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

    fn contains(&self, name: impl ConfigName) -> ConfigResult<bool> {
        name.with_config_name(|name| {
            let Some(key) = self.visible_property_key(name)? else {
                return Ok(false);
            };
            self.config.contains(key.as_ref())
        })
    }

    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: StrictValueRead,
    {
        name.with_config_name(|name| {
            let key = self
                .visible_property_key(name)?
                .ok_or_else(|| self.missing_property_error(name))?;
            self.config.get_strict(key.as_ref())
        })
    }

    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: StrictValueListRead,
    {
        name.with_config_name(|name| {
            let key = self
                .visible_property_key(name)?
                .ok_or_else(|| self.missing_property_error(name))?;
            self.config.get_list_strict(key.as_ref())
        })
    }

    fn contains_key_prefix(&self, prefix: &str) -> bool {
        self.visible_entries()
            .any(|(key, _)| key.starts_with(prefix))
    }

    fn contains_section(&self, path: &str) -> ConfigResult<bool> {
        ensure_config_path(path)?;
        if path.is_empty() {
            return Ok(self.visible_entries().next().is_some());
        }
        let child_prefix = format!("{path}.");
        Ok(self.contains_key_prefix(&child_prefix))
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
        Box::new(self.visible_entries())
    }

    #[inline(always)]
    fn section(&self, path: &str) -> ConfigResult<ConfigSection<'a>> {
        ConfigSection::section(self, path)
    }

    fn resolve_key(&self, name: impl ConfigName) -> ConfigResult<String> {
        name.with_config_name(|name| {
            ensure_config_path(name)?;
            Ok(self.resolve_key_cow(name).into_owned())
        })
    }
}
