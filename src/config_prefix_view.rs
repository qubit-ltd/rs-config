/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
#![allow(private_bounds)]

use std::borrow::Cow;

use qubit_value::multi_values::{MultiValuesFirstGetter, MultiValuesGetter};
use qubit_value::MultiValues;

use crate::config::Config;
use crate::config_reader::ConfigReader;
use crate::{ConfigResult, Property};

/// Read-only **prefix** view over a [`Config`]: key lookups use a logical key
/// prefix.
///
/// This type is named explicitly so other kinds of configuration views can be
/// added later without overloading a generic `ConfigView`.
///
/// Lookups rewrite keys by prepending `prefix`, while exposing keys relative to
/// that prefix.
#[derive(Debug, Clone)]
pub struct ConfigPrefixView<'a> {
    config: &'a Config,
    prefix: String,
    full_prefix: Option<String>,
}

impl<'a> ConfigPrefixView<'a> {
    /// Builds a prefix view for `config` with the given `prefix` (leading and
    /// trailing `.` are trimmed; empty means the root).
    ///
    /// # Parameters
    ///
    /// * `config` - Underlying configuration.
    /// * `prefix` - Logical prefix for relative keys.
    ///
    /// # Returns
    ///
    /// A new [`ConfigPrefixView`].
    pub(crate) fn new(config: &'a Config, prefix: &str) -> Self {
        let normalized_prefix = prefix.trim_matches('.').to_string();
        let full_prefix = if normalized_prefix.is_empty() {
            None
        } else {
            Some(format!("{normalized_prefix}."))
        };
        Self {
            config,
            prefix: normalized_prefix,
            full_prefix,
        }
    }

    /// Gets the logical prefix of this view.
    ///
    /// # Returns
    ///
    /// The normalized prefix string (no leading or trailing dot separators).
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Creates a nested prefix view by appending `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Segment to append (`.` is trimmed); empty keeps the current
    ///   prefix.
    ///
    /// # Returns
    ///
    /// A new view with the combined prefix.
    pub fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'a> {
        let child = prefix.trim_matches('.');
        if self.prefix.is_empty() {
            ConfigPrefixView::new(self.config, child)
        } else if child.is_empty() {
            ConfigPrefixView::new(self.config, self.prefix.as_str())
        } else {
            ConfigPrefixView::new(self.config, &format!("{}.{}", self.prefix, child))
        }
    }

    fn resolve_key<'b>(&'b self, name: &'b str) -> Cow<'b, str> {
        if self.prefix.is_empty() || name.is_empty() {
            return Cow::Borrowed(name);
        }
        if name == self.prefix {
            return Cow::Borrowed(name);
        }
        let full_prefix = self
            .full_prefix
            .as_deref()
            .expect("full_prefix must exist for non-empty prefix");
        if name.starts_with(full_prefix) {
            return Cow::Borrowed(name);
        }
        Cow::Owned(format!("{}.{}", self.prefix, name))
    }

    fn visible_entries<'b>(&'b self) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        let prefix = self.prefix.as_str();
        if prefix.is_empty() {
            return Box::new(self.config.properties.iter().map(|(k, v)| (k.as_str(), v)));
        }
        let full_prefix = self
            .full_prefix
            .as_deref()
            .expect("full_prefix must exist for non-empty prefix");
        Box::new(self.config.properties.iter().filter_map(move |(k, v)| {
            if k == prefix {
                Some((prefix, v))
            } else {
                k.strip_prefix(full_prefix).map(|stripped| (stripped, v))
            }
        }))
    }

    /// Combines this view's prefix with a relative `sub_prefix` for delegation
    /// to [`Config::subconfig`] / [`Config::deserialize`].
    fn effective_root_prefix(&self, sub_prefix: &str) -> String {
        let child = sub_prefix.trim_matches('.');
        if self.prefix.is_empty() {
            child.to_string()
        } else if child.is_empty() {
            self.prefix.clone()
        } else {
            format!("{}.{}", self.prefix, child)
        }
    }
}

impl<'a> ConfigReader for ConfigPrefixView<'a> {
    fn is_enable_variable_substitution(&self) -> bool {
        self.config.is_enable_variable_substitution()
    }

    fn max_substitution_depth(&self) -> usize {
        self.config.max_substitution_depth()
    }

    fn description(&self) -> Option<&str> {
        self.config.description()
    }

    fn get_property(&self, name: &str) -> Option<&Property> {
        let key = self.resolve_key(name);
        self.config.get_property(key.as_ref())
    }

    fn len(&self) -> usize {
        self.visible_entries().count()
    }

    fn is_empty(&self) -> bool {
        self.visible_entries().next().is_none()
    }

    fn keys(&self) -> Vec<String> {
        self.visible_entries()
            .map(|(k, _)| k.to_string())
            .collect()
    }

    fn contains(&self, name: &str) -> bool {
        let key = self.resolve_key(name);
        self.config.contains(key.as_ref())
    }

    fn get<T>(&self, name: &str) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        let key = self.resolve_key(name);
        self.config.get(key.as_ref())
    }

    fn get_list<T>(&self, name: &str) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        let key = self.resolve_key(name);
        self.config.get_list(key.as_ref())
    }

    fn get_optional<T>(&self, name: &str) -> ConfigResult<Option<T>>
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        let key = self.resolve_key(name);
        self.config.get_optional(key.as_ref())
    }

    fn get_optional_list<T>(&self, name: &str) -> ConfigResult<Option<Vec<T>>>
    where
        MultiValues: MultiValuesGetter<T>,
    {
        let key = self.resolve_key(name);
        self.config.get_optional_list(key.as_ref())
    }

    fn contains_prefix(&self, prefix: &str) -> bool {
        self.visible_entries().any(|(k, _)| k.starts_with(prefix))
    }

    fn iter_prefix<'b>(
        &'b self,
        prefix: &'b str,
    ) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        Box::new(
            self.visible_entries()
                .filter(move |(k, _)| k.starts_with(prefix)),
        )
    }

    fn iter<'b>(&'b self) -> Box<dyn Iterator<Item = (&'b str, &'b Property)> + 'b> {
        self.visible_entries()
    }

    fn is_null(&self, name: &str) -> bool {
        let key = self.resolve_key(name);
        self.config.is_null(key.as_ref())
    }

    fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config> {
        let full = self.effective_root_prefix(prefix);
        self.config.subconfig(&full, strip_prefix)
    }

    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let full = self.effective_root_prefix(prefix);
        self.config.deserialize(&full)
    }

    fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'a> {
        ConfigPrefixView::prefix_view(self, prefix)
    }
}
