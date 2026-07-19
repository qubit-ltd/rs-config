// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_datatype::{
    BlankStringPolicy,
    BooleanConversionOptions,
    CollectionConversionOptions,
    DataConversionOptions,
    DurationConversionOptions,
    EmptyItemPolicy,
    NumericConversionOptions,
    StringConversionOptions,
};
use serde::{
    Deserialize,
    Serialize,
};

use super::VariableSubstitutionOptions;

/// Runtime options that control how configuration values are read and parsed.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigReadOptions {
    /// Common scalar, collection, boolean, and duration conversion options.
    conversion: DataConversionOptions,
    /// Variable-substitution policy and resource limits.
    substitution: VariableSubstitutionOptions,
}

impl ConfigReadOptions {
    /// Creates options suitable for environment-variable style values.
    ///
    /// # Returns
    ///
    /// Options that trim strings, treat blank scalar strings as missing, accept
    /// common boolean aliases, and split scalar strings on commas while
    /// skipping empty collection items. Text-to-float conversion permits
    /// nearest-even rounding, while other numeric conversions remain exact.
    /// Environment-variable substitution is still disabled; enable it through
    /// [`VariableSubstitutionOptions`](super::VariableSubstitutionOptions).
    pub fn env_friendly() -> Self {
        Self {
            conversion: DataConversionOptions::env_friendly(),
            substitution: VariableSubstitutionOptions::default(),
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

    /// Gets the variable-substitution policy active for typed reads.
    ///
    /// # Returns
    ///
    /// Substitution behavior and resource limits.
    #[inline(always)]
    pub const fn substitution(&self) -> &VariableSubstitutionOptions {
        &self.substitution
    }

    /// Returns a copy with a different variable-substitution policy.
    ///
    /// # Parameters
    ///
    /// * `substitution` - Replacement behavior and resource limits.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub fn with_substitution(
        mut self,
        substitution: VariableSubstitutionOptions,
    ) -> Self {
        self.substitution = substitution;
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
    pub fn with_blank_string_policy(
        mut self,
        policy: BlankStringPolicy,
    ) -> Self {
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
    pub fn with_string_options(
        mut self,
        string: StringConversionOptions,
    ) -> Self {
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
    pub fn with_boolean_options(
        mut self,
        boolean: BooleanConversionOptions,
    ) -> Self {
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
    pub fn with_collection_options(
        mut self,
        collection: CollectionConversionOptions,
    ) -> Self {
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
    pub fn with_duration_options(
        mut self,
        duration: DurationConversionOptions,
    ) -> Self {
        self.conversion = self.conversion.with_duration_options(duration);
        self
    }

    /// Returns a copy with different numeric conversion options.
    ///
    /// # Parameters
    ///
    /// * `numeric` - New numeric conversion policies and resource limits.
    ///
    /// # Returns
    ///
    /// Updated options.
    pub fn with_numeric_options(
        mut self,
        numeric: NumericConversionOptions,
    ) -> Self {
        self.conversion = self.conversion.with_numeric_options(numeric);
        self
    }
}

impl AsRef<DataConversionOptions> for ConfigReadOptions {
    /// Borrows the underlying data conversion options.
    #[inline]
    fn as_ref(&self) -> &DataConversionOptions {
        &self.conversion
    }
}

impl From<DataConversionOptions> for ConfigReadOptions {
    /// Creates config read options from data conversion options.
    ///
    /// Environment-variable fallback for `${...}` substitution remains
    /// disabled.
    #[inline]
    fn from(conversion: DataConversionOptions) -> Self {
        Self {
            conversion,
            substitution: VariableSubstitutionOptions::default(),
        }
    }
}
