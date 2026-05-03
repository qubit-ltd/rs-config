/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

#![allow(private_bounds)]

use qubit_value::MultiValues;
use qubit_value::multi_values::{MultiValuesFirstGetter, MultiValuesGetter};
use serde::de::DeserializeOwned;

use crate::config_prefix_view::ConfigPrefixView;
use crate::field::ConfigField;
use crate::from::{
    FromConfig, IntoConfigDefault, is_effectively_missing,
    is_effectively_missing_with_substitution, parse_property_from_reader,
    parse_property_from_reader_with_substitution,
};
use crate::options::ConfigReadOptions;
use crate::{Config, ConfigError, ConfigName, ConfigNames, ConfigResult, Property};

/// Read-only configuration interface.
///
/// This trait allows consumers to read configuration values without requiring
/// ownership of a [`crate::Config`]. Both [`crate::Config`] and
/// [`crate::ConfigPrefixView`] implement it.
///
/// Its required methods mirror the read-only surface of [`crate::Config`]
/// (metadata, raw properties, iteration, subtree extraction, and serde
/// deserialization), with prefix views resolving keys relative to their
/// logical prefix.
///
pub trait ConfigReader {
    /// Returns whether `${...}` variable substitution is applied when reading
    /// string values.
    ///
    /// # Returns
    ///
    /// `true` if substitution is enabled for this reader.
    fn is_enable_variable_substitution(&self) -> bool;

    /// Returns the maximum recursion depth allowed when resolving nested
    /// `${...}` references.
    ///
    /// # Returns
    ///
    /// Maximum substitution depth (see
    /// `DEFAULT_MAX_SUBSTITUTION_DEPTH` for the default used by
    /// [`crate::Config`]).
    fn max_substitution_depth(&self) -> usize;

    /// Returns the optional human-readable description attached to this
    /// configuration (the whole document; prefix views expose the same value
    /// as the underlying [`crate::Config`]).
    fn description(&self) -> Option<&str>;

    /// Returns a reference to the raw [`Property`] for `name`, if present.
    ///
    /// For a [`ConfigPrefixView`], `name` is resolved relative to the view
    /// prefix (same rules as [`Self::get`]).
    fn get_property(&self, name: impl ConfigName) -> Option<&Property>;

    /// Number of configuration entries visible to this reader (all keys for
    /// [`crate::Config`]; relative keys only for a [`ConfigPrefixView`]).
    fn len(&self) -> usize;

    /// Returns `true` when [`Self::len`] is zero.
    fn is_empty(&self) -> bool;

    /// All keys visible to this reader (relative keys for a prefix view).
    fn keys(&self) -> Vec<String>;

    /// Returns whether a property exists for the given key.
    ///
    /// # Parameters
    ///
    /// * `name` - Full configuration key (for [`crate::ConfigPrefixView`],
    ///   relative keys are resolved against the view prefix).
    ///
    /// # Returns
    ///
    /// `true` if the key is present.
    fn contains(&self, name: impl ConfigName) -> bool;

    /// Reads the first stored value for `name` and converts it to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The converted value on success, or a [`crate::ConfigError`] if the key
    /// is missing, empty, or not convertible.
    fn get<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            let property = self
                .get_property(name)
                .ok_or_else(|| ConfigError::PropertyNotFound(resolved.clone()))?;
            if !property.is_empty()
                && is_effectively_missing(self, &resolved, property, self.read_options())?
            {
                return Err(ConfigError::PropertyHasNoValue(resolved));
            }
            parse_property_from_reader(self, &resolved, property, self.read_options())
        })
    }

    /// Reads the first stored value for `name` without cross-type conversion.
    ///
    /// # Type parameters
    ///
    /// * `T` - Exact target type; requires `MultiValues` to implement
    ///   `MultiValuesFirstGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The exact stored value on success, or a [`crate::ConfigError`] if the
    /// key is missing, empty, or has a different stored type.
    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>;

    /// Reads all stored values for `name` and converts each element to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type supported by the shared conversion layer.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of values on success, or a [`crate::ConfigError`] on failure.
    fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: FromConfig;

    /// Reads all stored values for `name` without cross-type conversion.
    ///
    /// # Type parameters
    ///
    /// * `T` - Exact element type; requires `MultiValues` to implement
    ///   `MultiValuesGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of exact stored values on success, or a
    /// [`crate::ConfigError`] on failure.
    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>;

    /// Gets a value or `default` if the key is missing or empty.
    ///
    /// Conversion and substitution errors are returned instead of being hidden by
    /// the default.
    #[inline]
    fn get_or<T>(
        &self,
        name: impl ConfigName,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        self.get_optional(name)
            .map(|value| value.unwrap_or_else(|| default.into_config_default()))
    }

    /// Gets an optional value with the same semantics as [`crate::Config::get_optional`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key (relative for a prefix view).
    ///
    /// # Returns
    ///
    /// `Ok(Some(v))`, `Ok(None)` when missing or empty, or `Err` on conversion failure.
    fn get_optional<T>(&self, name: impl ConfigName) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            match self.get_property(name) {
                None => Ok(None),
                Some(property)
                    if is_effectively_missing(self, &resolved, property, self.read_options())? =>
                {
                    Ok(None)
                }
                Some(property) => {
                    parse_property_from_reader(self, &resolved, property, self.read_options())
                        .map(Some)
                }
            }
        })
    }

    /// Gets the read options active for this reader.
    ///
    /// # Returns
    ///
    /// Global read options inherited by field-less reads.
    fn read_options(&self) -> &ConfigReadOptions;

    /// Reads a value from the first present and non-empty key in `names`.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// Parsed value from the first configured key. Conversion errors stop the
    /// search and are returned directly.
    fn get_any<T>(&self, names: impl ConfigNames) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any(names)?.ok_or_else(|| {
                ConfigError::PropertyNotFound(format!("one of: {}", names.join(", ")))
            })
        })
    }

    /// Reads an optional value from the first present and non-empty key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` only when all keys are missing or empty.
    fn get_optional_any<T>(&self, names: impl ConfigNames) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any_with_options(names, self.read_options())
        })
    }

    /// Reads a value from any key, using `default` only when all keys are
    /// absent or empty.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    /// * `default` - Fallback when no candidate is configured.
    ///
    /// # Returns
    ///
    /// Parsed value or `default`; parsing errors are never swallowed.
    fn get_any_or<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any(names)
                .map(|value| value.unwrap_or_else(|| default.into_config_default()))
        })
    }

    /// Reads a value from any key with explicit read options, using `default`
    /// only when all keys are absent or empty.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    /// * `default` - Fallback when no candidate is configured.
    /// * `read_options` - Parsing options for this read.
    ///
    /// # Returns
    ///
    /// Parsed value or `default`; parsing errors are never swallowed.
    fn get_any_or_with<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
        read_options: &ConfigReadOptions,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any_with_options(names, read_options)
                .map(|value| value.unwrap_or_else(|| default.into_config_default()))
        })
    }

    /// Reads a declared field.
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration containing name, aliases, defaults, and
    ///   optional field-level read options.
    ///
    /// # Returns
    ///
    /// Parsed field value or its default.
    fn read<T>(&self, field: ConfigField<T>) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        let ConfigField {
            name,
            aliases,
            default,
            read_options,
        } = field;
        let options = read_options.as_ref().unwrap_or_else(|| self.read_options());
        let mut names = Vec::with_capacity(1 + aliases.len());
        names.push(name.as_str());
        names.extend(aliases.iter().map(String::as_str));
        self.get_optional_any_with_options(&names, options)?
            .or(default)
            .ok_or_else(|| ConfigError::PropertyNotFound(format!("one of: {}", names.join(", "))))
    }

    /// Reads an optional declared field.
    ///
    /// # Parameters
    ///
    /// * `field` - Field declaration.
    ///
    /// # Returns
    ///
    /// Parsed field value, its default, or `None`.
    fn read_optional<T>(&self, field: ConfigField<T>) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        let ConfigField {
            name,
            aliases,
            default,
            read_options,
        } = field;
        let options = read_options.as_ref().unwrap_or_else(|| self.read_options());
        let mut names = Vec::with_capacity(1 + aliases.len());
        names.push(name.as_str());
        names.extend(aliases.iter().map(String::as_str));
        self.get_optional_any_with_options(&names, options)
            .map(|value| value.or(default))
    }

    /// Shared implementation for field-level and global multi-key reads.
    fn get_optional_any_with_options<T>(
        &self,
        names: impl ConfigNames,
        options: &ConfigReadOptions,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            for name in names {
                let Some(property) = self.get_property(*name) else {
                    continue;
                };
                let resolved = self.resolve_key(*name);
                if is_effectively_missing(self, &resolved, property, options)? {
                    continue;
                }
                return parse_property_from_reader(self, &resolved, property, options).map(Some);
            }
            Ok(None)
        })
    }

    /// Gets an optional list with the same semantics as [`crate::Config::get_optional_list`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type supported by the shared conversion layer.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, `Ok(None)` when missing or empty, or `Err` on failure.
    fn get_optional_list<T>(&self, name: impl ConfigName) -> ConfigResult<Option<Vec<T>>>
    where
        T: FromConfig;

    /// Returns whether any key visible to this reader starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix to test (for a prefix view, keys are relative to
    ///   that view).
    ///
    /// # Returns
    ///
    /// `true` if at least one matching key exists.
    fn contains_prefix(&self, prefix: &str) -> bool;

    /// Iterates `(key, property)` pairs for keys that start with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix filter.
    ///
    /// # Returns
    ///
    /// A boxed iterator over matching entries.
    fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a>;

    /// Iterates all `(key, property)` pairs visible to this reader (same scope
    /// as [`Self::keys`]).
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a>;

    /// Returns `true` if the key exists and the property has no values (same
    /// as [`crate::Config::is_null`]).
    fn is_null(&self, name: impl ConfigName) -> bool;

    /// Extracts a subtree as a new [`Config`] (same semantics as
    /// [`crate::Config::subconfig`]; on a prefix view, `prefix` is relative to
    /// the view).
    fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config>;

    /// Deserializes the subtree at `prefix` with serde (same as
    /// [`crate::Config::deserialize`]; on a prefix view, `prefix` is relative).
    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned;

    /// Creates a read-only prefix view; relative keys resolve under `prefix`.
    ///
    /// Semantics match [`crate::Config::prefix_view`] and
    /// [`crate::ConfigPrefixView::prefix_view`] (nested prefix when called on a
    /// view).
    ///
    /// # Parameters
    ///
    /// * `prefix` - Logical prefix; empty means the full configuration (same as
    ///   root).
    ///
    /// # Returns
    ///
    /// A [`ConfigPrefixView`] borrowing this reader's underlying
    /// [`crate::Config`].
    fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'_>;

    /// Resolves `name` into the canonical key path against the root
    /// [`crate::Config`].
    ///
    /// For a root [`crate::Config`], this returns `name` unchanged. For a
    /// [`crate::ConfigPrefixView`], this prepends the effective view prefix so
    /// callers can report root-relative key paths in diagnostics.
    ///
    /// # Parameters
    ///
    /// * `name` - Relative or absolute key in the current reader scope.
    ///
    /// # Returns
    ///
    /// Root-relative key path string.
    #[inline]
    fn resolve_key(&self, name: impl ConfigName) -> String {
        name.with_config_name(str::to_string)
    }

    /// Gets a string value, applying variable substitution when enabled.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The string after `${...}` resolution, or a [`crate::ConfigError`].
    fn get_string(&self, name: impl ConfigName) -> ConfigResult<String> {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            let property = self
                .get_property(name)
                .ok_or_else(|| ConfigError::PropertyNotFound(resolved.clone()))?;
            if !property.is_empty()
                && is_effectively_missing_with_substitution(
                    self,
                    &resolved,
                    property,
                    self.read_options(),
                )?
            {
                return Err(ConfigError::PropertyHasNoValue(resolved));
            }
            parse_property_from_reader_with_substitution(
                self,
                &resolved,
                property,
                self.read_options(),
            )
        })
    }

    /// Gets a string value from the first present and non-empty key in `names`.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// The resolved string from the first configured key.
    #[inline]
    fn get_string_any(&self, names: impl ConfigNames) -> ConfigResult<String> {
        names.with_config_names(|names| {
            self.get_optional_string_any(names)?.ok_or_else(|| {
                ConfigError::PropertyNotFound(format!("one of: {}", names.join(", ")))
            })
        })
    }

    /// Gets an optional string value from the first present and non-empty key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` only when all keys are missing or empty.
    #[inline]
    fn get_optional_string_any(&self, names: impl ConfigNames) -> ConfigResult<Option<String>> {
        names.with_config_names(|names| {
            self.get_optional_any_with_options_and_substitution(names, self.read_options())
        })
    }

    /// Gets a string from any key, or `default` when all keys are missing or
    /// empty.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    /// * `default` - Fallback string used only when every key is missing or
    ///   empty.
    ///
    /// # Returns
    ///
    /// The resolved string or a clone of `default`; substitution errors are
    /// returned.
    #[inline]
    fn get_string_any_or(&self, names: impl ConfigNames, default: &str) -> ConfigResult<String> {
        names.with_config_names(|names| {
            self.get_optional_any_with_options_and_substitution(names, self.read_options())
                .map(|value| value.unwrap_or_else(|| default.to_string()))
        })
    }

    /// Gets a string value with substitution, or `default` if the key is
    /// missing or empty.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    /// * `default` - Fallback string used only when the key is missing or empty.
    ///
    /// # Returns
    ///
    /// The resolved string or a clone of `default`; parsing and substitution
    /// errors are returned.
    #[inline]
    fn get_string_or(&self, name: impl ConfigName, default: &str) -> ConfigResult<String> {
        self.get_optional_string(name)
            .map(|value| value.unwrap_or_else(|| default.to_string()))
    }

    /// Gets all string values for `name`, applying substitution to each element
    /// when enabled.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of resolved strings, or a [`crate::ConfigError`].
    fn get_string_list(&self, name: impl ConfigName) -> ConfigResult<Vec<String>> {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            let property = self
                .get_property(name)
                .ok_or_else(|| ConfigError::PropertyNotFound(resolved.clone()))?;
            if !property.is_empty()
                && is_effectively_missing_with_substitution(
                    self,
                    &resolved,
                    property,
                    self.read_options(),
                )?
            {
                return Err(ConfigError::PropertyHasNoValue(resolved));
            }
            parse_property_from_reader_with_substitution(
                self,
                &resolved,
                property,
                self.read_options(),
            )
        })
    }

    /// Gets a string list with substitution, or copies `default` if the key is
    /// missing or empty.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    /// * `default` - Fallback string slices used only when the key is missing or
    ///   empty.
    ///
    /// # Returns
    ///
    /// The resolved list or `default` converted to owned `String`s`; parsing and
    /// substitution errors are returned.
    #[inline]
    fn get_string_list_or(
        &self,
        name: impl ConfigName,
        default: &[&str],
    ) -> ConfigResult<Vec<String>> {
        self.get_optional_string_list(name).map(|value| {
            value.unwrap_or_else(|| default.iter().map(|item| (*item).to_string()).collect())
        })
    }

    /// Gets an optional string with the same three-way semantics as
    /// [`crate::Config::get_optional_string`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(None)` if the key is missing or empty; `Ok(Some(s))` with
    /// substitution applied; or `Err` if the value exists but cannot be read as
    /// a string.
    #[inline]
    fn get_optional_string(&self, name: impl ConfigName) -> ConfigResult<Option<String>> {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            match self.get_property(name) {
                None => Ok(None),
                Some(property)
                    if is_effectively_missing_with_substitution(
                        self,
                        &resolved,
                        property,
                        self.read_options(),
                    )? =>
                {
                    Ok(None)
                }
                Some(property) => parse_property_from_reader_with_substitution(
                    self,
                    &resolved,
                    property,
                    self.read_options(),
                )
                .map(Some),
            }
        })
    }

    /// Gets an optional string list with per-element substitution when enabled.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(None)` if the key is missing or empty; `Ok(Some(vec))` otherwise; or
    /// `Err` on conversion/substitution failure.
    #[inline]
    fn get_optional_string_list(&self, name: impl ConfigName) -> ConfigResult<Option<Vec<String>>> {
        name.with_config_name(|name| {
            let resolved = self.resolve_key(name);
            match self.get_property(name) {
                None => Ok(None),
                Some(property)
                    if is_effectively_missing_with_substitution(
                        self,
                        &resolved,
                        property,
                        self.read_options(),
                    )? =>
                {
                    Ok(None)
                }
                Some(property) => parse_property_from_reader_with_substitution(
                    self,
                    &resolved,
                    property,
                    self.read_options(),
                )
                .map(Some),
            }
        })
    }

    /// Shared implementation for string helper multi-key reads.
    fn get_optional_any_with_options_and_substitution<T>(
        &self,
        names: impl ConfigNames,
        options: &ConfigReadOptions,
    ) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            for name in names {
                let Some(property) = self.get_property(*name) else {
                    continue;
                };
                let resolved = self.resolve_key(*name);
                if is_effectively_missing_with_substitution(self, &resolved, property, options)? {
                    continue;
                }
                return parse_property_from_reader_with_substitution(
                    self, &resolved, property, options,
                )
                .map(Some);
            }
            Ok(None)
        })
    }
}
