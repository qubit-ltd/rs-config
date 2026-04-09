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

use crate::config_prefix_view::ConfigPrefixView;
use crate::{utils, ConfigResult, Property};

/// Read-only configuration interface.
///
/// This trait allows consumers to read configuration values without requiring
/// ownership of a [`crate::Config`]. Both [`crate::Config`] and [`crate::ConfigPrefixView`]
/// implement it.
///
/// Author: Haixing Hu
pub trait ConfigReader {
    /// Returns whether variable substitution is enabled.
    fn is_enable_variable_substitution(&self) -> bool;

    /// Returns maximum depth for variable substitution.
    fn max_substitution_depth(&self) -> usize;

    /// Returns `true` if key exists.
    fn contains(&self, name: &str) -> bool;

    /// Gets a typed value by key.
    fn get<T>(&self, name: &str) -> ConfigResult<T>
    where
        MultiValues: MultiValuesFirstGetter<T>;

    /// Gets all typed values by key.
    fn get_list<T>(&self, name: &str) -> ConfigResult<Vec<T>>
    where
        MultiValues: MultiValuesGetter<T>;

    /// Returns `true` if any visible key starts with `prefix`.
    fn contains_prefix(&self, prefix: &str) -> bool;

    /// Iterates visible keys that start with `prefix`.
    fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a>;

    /// Creates a read-only prefix view; relative keys resolve under `prefix`.
    ///
    /// Semantics match [`crate::Config::prefix_view`] and
    /// [`crate::ConfigPrefixView::prefix_view`] (nested prefix when called on a view).
    fn prefix_view(&self, prefix: &str) -> ConfigPrefixView<'_>;

    /// Gets a string value with variable substitution.
    fn get_string(&self, name: &str) -> ConfigResult<String> {
        let value: String = self.get(name)?;
        if self.is_enable_variable_substitution() {
            utils::substitute_variables(&value, self, self.max_substitution_depth())
        } else {
            Ok(value)
        }
    }

    /// Gets string value or default.
    fn get_string_or(&self, name: &str, default: &str) -> String {
        self.get_string(name)
            .unwrap_or_else(|_| default.to_string())
    }

    /// Gets a list of string values with variable substitution.
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

    /// Gets string list or default.
    fn get_string_list_or(&self, name: &str, default: &[&str]) -> Vec<String> {
        self.get_string_list(name)
            .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
    }

    /// Gets optional string value.
    fn get_optional_string(&self, name: &str) -> ConfigResult<Option<String>> {
        if self.contains(name) {
            Ok(Some(self.get_string(name)?))
        } else {
            Ok(None)
        }
    }

    /// Gets optional string list value.
    fn get_optional_string_list(&self, name: &str) -> ConfigResult<Option<Vec<String>>> {
        if self.contains(name) {
            Ok(Some(self.get_string_list(name)?))
        } else {
            Ok(None)
        }
    }
}
