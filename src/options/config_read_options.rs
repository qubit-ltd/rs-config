/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::ops::Deref;

use qubit_datatype::{
    BlankStringPolicy, BooleanConversionOptions, CollectionConversionOptions,
    DataConversionOptions, DurationConversionOptions, EmptyItemPolicy, StringConversionOptions,
};

/// Runtime options that control how configuration values are read and parsed.
///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigReadOptions {
    /// Common scalar, collection, boolean, and duration conversion options.
    conversion: DataConversionOptions,
    /// Whether unresolved `${...}` placeholders may be read from process
    /// environment variables.
    env_variable_substitution_enabled: bool,
}

impl ConfigReadOptions {
    /// Creates options suitable for environment-variable style values.
    ///
    /// # Returns
    ///
    /// Options that trim strings, treat blank scalar strings as missing, accept
    /// common boolean aliases, and split scalar strings on commas while
    /// skipping empty collection items. Environment-variable substitution is
    /// still disabled; enable it explicitly with
    /// [`Self::with_env_variable_substitution_enabled`].
    #[must_use]
    pub fn env_friendly() -> Self {
        Self {
            conversion: DataConversionOptions::env_friendly(),
            env_variable_substitution_enabled: false,
        }
    }

    /// Gets the underlying data conversion options.
    ///
    /// # Returns
    ///
    /// Options used by the shared `qubit-datatype` conversion layer.
    #[inline]
    pub fn conversion_options(&self) -> &DataConversionOptions {
        &self.conversion
    }

    /// Returns whether `${...}` substitution may read process environment
    /// variables when a value is missing from config.
    ///
    /// # Returns
    ///
    /// `true` when environment fallback is enabled.
    #[inline]
    pub fn is_env_variable_substitution_enabled(&self) -> bool {
        self.env_variable_substitution_enabled
    }

    /// Returns a copy with environment-variable substitution enabled or
    /// disabled.
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether unresolved config placeholders may fall back to
    ///   process environment variables.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_env_variable_substitution_enabled(mut self, enabled: bool) -> Self {
        self.env_variable_substitution_enabled = enabled;
        self
    }

    /// Returns a copy with a different blank string policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - New blank string policy.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_blank_string_policy(mut self, policy: BlankStringPolicy) -> Self {
        self.conversion = self.conversion.with_blank_string_policy(policy);
        self
    }

    /// Returns a copy with a different empty collection item policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - New empty item policy.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_empty_item_policy(mut self, policy: EmptyItemPolicy) -> Self {
        self.conversion = self.conversion.with_empty_item_policy(policy);
        self
    }

    /// Returns a copy with different string conversion options.
    ///
    /// # Parameters
    ///
    /// * `string` - New string conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_string_options(mut self, string: StringConversionOptions) -> Self {
        self.conversion = self.conversion.with_string_options(string);
        self
    }

    /// Returns a copy with different boolean conversion options.
    ///
    /// # Parameters
    ///
    /// * `boolean` - New boolean conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_boolean_options(mut self, boolean: BooleanConversionOptions) -> Self {
        self.conversion = self.conversion.with_boolean_options(boolean);
        self
    }

    /// Returns a copy with different collection conversion options.
    ///
    /// # Parameters
    ///
    /// * `collection` - New collection conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_collection_options(mut self, collection: CollectionConversionOptions) -> Self {
        self.conversion = self.conversion.with_collection_options(collection);
        self
    }

    /// Returns a copy with different duration conversion options.
    ///
    /// # Parameters
    ///
    /// * `duration` - New duration conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_duration_options(mut self, duration: DurationConversionOptions) -> Self {
        self.conversion = self.conversion.with_duration_options(duration);
        self
    }
}

impl Deref for ConfigReadOptions {
    type Target = DataConversionOptions;

    /// Dereferences to the underlying data conversion options.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.conversion
    }
}
