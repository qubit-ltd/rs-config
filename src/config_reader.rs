/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
#![allow(private_bounds)]

use qubit_value::multi_values::{MultiValuesFirstGetter, MultiValuesGetter};
use qubit_value::MultiValues;
use serde::de::DeserializeOwned;

use crate::config_prefix_view::ConfigPrefixView;
use crate::{utils, Config, ConfigResult, Property};

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
/// Author: Haixing Hu
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
    /// [`crate::constants::DEFAULT_MAX_SUBSTITUTION_DEPTH`] for the default
    /// used by [`crate::Config`]).
    fn max_substitution_depth(&self) -> usize;

    /// Returns the optional human-readable description attached to this
    /// configuration (the whole document; prefix views expose the same value
    /// as the underlying [`crate::Config`]).
    fn description(&self) -> Option<&str>;

    /// Returns a reference to the raw [`Property`] for `name`, if present.
    ///
    /// For a [`ConfigPrefixView`], `name` is resolved relative to the view
    /// prefix (same rules as [`Self::get`]).
    fn get_property(&self, name: &str) -> Option<&Property>;

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
    fn contains(&self, name: &str) -> bool;

    /// Reads the first stored value for `name` and converts it to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type; requires `MultiValues` to implement
    ///   `MultiValuesFirstGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The converted value on success, or a [`crate::ConfigError`] if the key
    /// is missing, empty, or not convertible.
    fn get<T>(&self, name: &str) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>;

    /// Reads all stored values for `name` and converts each element to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type; requires `MultiValues` to implement
    ///   `MultiValuesGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of values on success, or a [`crate::ConfigError`] on failure.
    fn get_list<T>(&self, name: &str) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>;

    /// Gets a value or `default` if the key is missing or conversion fails (same
    /// as [`crate::Config::get_or`]).
    fn get_or<T>(&self, name: &str, default: T) -> T
    where
        MultiValues: MultiValuesFirstGetter<T>,
    {
        self.get(name).unwrap_or(default)
    }

    /// Gets an optional value with the same semantics as [`crate::Config::get_optional`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type; requires `MultiValues` to implement
    ///   `MultiValuesFirstGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key (relative for a prefix view).
    ///
    /// # Returns
    ///
    /// `Ok(Some(v))`, `Ok(None)` when missing or empty, or `Err` on conversion failure.
    fn get_optional<T>(&self, name: &str) -> ConfigResult<Option<T>>
    where
        MultiValues: MultiValuesFirstGetter<T>;

    /// Gets an optional list with the same semantics as [`crate::Config::get_optional_list`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type; requires `MultiValues` to implement `MultiValuesGetter` for `T`.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, `Ok(None)` when missing or empty, or `Err` on failure.
    fn get_optional_list<T>(&self, name: &str) -> ConfigResult<Option<Vec<T>>>
    where
        MultiValues: MultiValuesGetter<T>;

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
    fn is_null(&self, name: &str) -> bool;

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

    /// Gets a string value, applying variable substitution when enabled.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The string after `${...}` resolution, or a [`crate::ConfigError`].
    fn get_string(&self, name: &str) -> ConfigResult<String> {
        let value: String = self.get(name)?;
        if self.is_enable_variable_substitution() {
            utils::substitute_variables(&value, self, self.max_substitution_depth())
        } else {
            Ok(value)
        }
    }

    /// Gets a string value with substitution, or `default` if lookup or
    /// substitution fails.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    /// * `default` - Fallback string when [`Self::get_string`] would error.
    ///
    /// # Returns
    ///
    /// The resolved string or a clone of `default`.
    fn get_string_or(&self, name: &str, default: &str) -> String {
        self.get_string(name)
            .unwrap_or_else(|_| default.to_string())
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
    fn get_string_list(&self, name: &str) -> ConfigResult<Vec<String>> {
        let values: Vec<String> = self.get_list(name)?;
        if self.is_enable_variable_substitution() {
            values
                .into_iter()
                .map(|v| utils::substitute_variables(&v, self, self.max_substitution_depth()))
                .collect()
        } else {
            Ok(values)
        }
    }

    /// Gets a string list with substitution, or copies `default` if lookup
    /// fails.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    /// * `default` - Fallback string slices when [`Self::get_string_list`]
    ///   would error.
    ///
    /// # Returns
    ///
    /// The resolved list or `default` converted to owned `String`s.
    fn get_string_list_or(&self, name: &str, default: &[&str]) -> Vec<String> {
        self.get_string_list(name)
            .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
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
    fn get_optional_string(&self, name: &str) -> ConfigResult<Option<String>> {
        match self.get_property(name) {
            None => Ok(None),
            Some(prop) if prop.is_empty() => Ok(None),
            Some(_) => self.get_string(name).map(Some),
        }
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
    fn get_optional_string_list(&self, name: &str) -> ConfigResult<Option<Vec<String>>> {
        match self.get_property(name) {
            None => Ok(None),
            Some(prop) if prop.is_empty() => Ok(None),
            Some(_) => self.get_string_list(name).map(Some),
        }
    }
}
