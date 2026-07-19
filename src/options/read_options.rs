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

use crate::constants::{
    DEFAULT_MAX_SUBSTITUTION_DEPTH,
    DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
    DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
};

/// Runtime options that control configuration conversion and interpolation.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadOptions {
    /// Common scalar, collection, boolean, and duration conversion options.
    conversion: DataConversionOptions,
    /// Whether unresolved placeholders may use process environment variables.
    environment_fallback_enabled: bool,
    /// Maximum active placeholder-reference chain length.
    max_interpolation_depth: usize,
    /// Maximum number of placeholder resolutions in one read.
    max_interpolation_expansions: usize,
    /// Maximum UTF-8 byte length of one interpolated value.
    max_interpolation_output_bytes: usize,
}

impl ReadOptions {
    /// Creates options suitable for environment-variable style values.
    ///
    /// # Returns
    ///
    /// Options that trim strings, treat blank scalar strings as missing, accept
    /// common boolean aliases, and split scalar strings on commas while
    /// skipping empty collection items. Interpolated reads may fall back to
    /// process environment variables.
    pub fn env_friendly() -> Self {
        Self {
            conversion: DataConversionOptions::env_friendly(),
            ..Self::default()
        }
    }

    /// Gets the underlying data conversion options.
    ///
    /// # Returns
    ///
    /// Options used by the shared `qubit-datatype` conversion layer.
    #[inline(always)]
    pub const fn conversion_options(&self) -> &DataConversionOptions {
        &self.conversion
    }

    /// Returns whether unresolved placeholders may use the environment.
    ///
    /// # Returns
    ///
    /// `true` when interpolated reads may fall back to process environment
    /// variables.
    #[inline(always)]
    pub const fn is_environment_fallback_enabled(&self) -> bool {
        self.environment_fallback_enabled
    }

    /// Returns the maximum recursive interpolation depth.
    ///
    /// # Returns
    ///
    /// The maximum active placeholder-reference chain length.
    #[inline(always)]
    pub const fn max_interpolation_depth(&self) -> usize {
        self.max_interpolation_depth
    }

    /// Returns the maximum placeholder-resolution count per read.
    ///
    /// # Returns
    ///
    /// The configured interpolation expansion-count limit.
    #[inline(always)]
    pub const fn max_interpolation_expansions(&self) -> usize {
        self.max_interpolation_expansions
    }

    /// Returns the maximum interpolated UTF-8 byte length.
    ///
    /// # Returns
    ///
    /// The configured output-size limit in bytes.
    #[inline(always)]
    pub const fn max_interpolation_output_bytes(&self) -> usize {
        self.max_interpolation_output_bytes
    }

    /// Returns a copy with environment fallback enabled or disabled.
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether unresolved placeholders may use environment
    ///   values.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_environment_fallback_enabled(
        mut self,
        enabled: bool,
    ) -> Self {
        self.environment_fallback_enabled = enabled;
        self
    }

    /// Returns a copy with a different recursive interpolation-depth limit.
    ///
    /// # Parameters
    ///
    /// * `max_depth` - Maximum active placeholder-reference chain length.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_interpolation_depth(
        mut self,
        max_depth: usize,
    ) -> Self {
        self.max_interpolation_depth = max_depth;
        self
    }

    /// Returns a copy with a different placeholder-resolution limit.
    ///
    /// # Parameters
    ///
    /// * `max_expansions` - Maximum resolved placeholders per read.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_interpolation_expansions(
        mut self,
        max_expansions: usize,
    ) -> Self {
        self.max_interpolation_expansions = max_expansions;
        self
    }

    /// Returns a copy with a different interpolated-output byte limit.
    ///
    /// # Parameters
    ///
    /// * `max_output_bytes` - Maximum UTF-8 bytes in one interpolated value.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_interpolation_output_bytes(
        mut self,
        max_output_bytes: usize,
    ) -> Self {
        self.max_interpolation_output_bytes = max_output_bytes;
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

impl Default for ReadOptions {
    /// Creates the default conversion and bounded interpolation policy.
    #[inline]
    fn default() -> Self {
        Self {
            conversion: DataConversionOptions::default(),
            environment_fallback_enabled: true,
            max_interpolation_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            max_interpolation_expansions: DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
            max_interpolation_output_bytes:
                DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
        }
    }
}

impl AsRef<DataConversionOptions> for ReadOptions {
    /// Borrows the underlying data conversion options.
    #[inline(always)]
    fn as_ref(&self) -> &DataConversionOptions {
        &self.conversion
    }
}

impl From<DataConversionOptions> for ReadOptions {
    /// Creates read options from data conversion options.
    #[inline]
    fn from(conversion: DataConversionOptions) -> Self {
        Self {
            conversion,
            ..Self::default()
        }
    }
}
