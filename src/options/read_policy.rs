// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_datatype::{
    BlankStringPolicy, BooleanConversionOptions, CollectionConversionOptions,
    DataConversionOptions, DurationConversionOptions, EmptyItemPolicy, NumericConversionOptions,
    StringConversionOptions,
};
use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_MAX_SUBSTITUTION_DEPTH, DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
    DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
};

/// Sources consulted while resolving an interpolated variable.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolationSources {
    /// Resolve variables from configuration data only.
    ConfigOnly,
    /// Resolve from configuration data, then fall back to the process
    /// environment.
    ConfigThenEnv,
}

impl Default for InterpolationSources {
    /// Uses configuration data as the only interpolation source.
    #[inline]
    fn default() -> Self {
        Self::ConfigOnly
    }
}

/// Runtime policy that controls configuration conversion and interpolation.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadPolicy {
    /// Common scalar, collection, boolean, and duration conversion options.
    conversion: DataConversionOptions,
    /// Sources consulted when resolving interpolated placeholders.
    interpolation_sources: InterpolationSources,
    /// Maximum active placeholder-reference chain length.
    max_interpolation_depth: usize,
    /// Maximum number of placeholder resolutions in one read.
    max_interpolation_expansions: usize,
    /// Maximum UTF-8 byte length of one interpolated value.
    max_interpolation_output_bytes: usize,
}

impl ReadPolicy {
    /// Creates a policy that resolves placeholders from configuration only.
    ///
    /// # Returns
    ///
    /// Default conversion and interpolation limits with process-environment
    /// fallback disabled. Use this preset when configuration content is not
    /// trusted to select arbitrary environment variable names.
    #[inline]
    pub fn config_only() -> Self {
        Self::default()
    }

    /// Creates a policy suitable for environment-variable style values.
    ///
    /// # Returns
    ///
    /// The returned policy changes only conversion behavior. Interpolated
    /// reads still resolve from configuration data unless callers explicitly
    /// select [`InterpolationSources::ConfigThenEnv`].
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

    /// Returns the configured interpolation sources.
    ///
    /// # Returns
    ///
    /// The source order used by interpolated reads.
    #[inline(always)]
    pub const fn interpolation_sources(&self) -> InterpolationSources {
        self.interpolation_sources
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

    /// Returns a copy with different interpolation sources.
    ///
    /// # Parameters
    ///
    /// * `sources` - Sources consulted for unresolved placeholders.
    ///
    /// # Returns
    ///
    /// Updated policy.
    #[inline(always)]
    pub const fn with_interpolation_sources(mut self, sources: InterpolationSources) -> Self {
        self.interpolation_sources = sources;
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
    /// Updated policy.
    #[inline(always)]
    pub const fn with_max_interpolation_depth(mut self, max_depth: usize) -> Self {
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
    /// Updated policy.
    #[inline(always)]
    pub const fn with_max_interpolation_expansions(mut self, max_expansions: usize) -> Self {
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
    /// Updated policy.
    #[inline(always)]
    pub const fn with_max_interpolation_output_bytes(mut self, max_output_bytes: usize) -> Self {
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
    /// Updated policy.
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
    /// Updated policy.
    pub fn with_empty_item_policy(mut self, policy: EmptyItemPolicy) -> Self {
        self.conversion = self.conversion.with_empty_item_policy(policy);
        self
    }

    /// Returns a copy with different string conversion options.
    ///
    /// # Parameters
    ///
    /// * `string` - New string conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_string_options(mut self, string: StringConversionOptions) -> Self {
        self.conversion = self.conversion.with_string_options(string);
        self
    }

    /// Returns a copy with different boolean conversion options.
    ///
    /// # Parameters
    ///
    /// * `boolean` - New boolean conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_boolean_options(mut self, boolean: BooleanConversionOptions) -> Self {
        self.conversion = self.conversion.with_boolean_options(boolean);
        self
    }

    /// Returns a copy with different collection conversion options.
    ///
    /// # Parameters
    ///
    /// * `collection` - New collection conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_collection_options(mut self, collection: CollectionConversionOptions) -> Self {
        self.conversion = self.conversion.with_collection_options(collection);
        self
    }

    /// Returns a copy with different duration conversion options.
    ///
    /// # Parameters
    ///
    /// * `duration` - New duration conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_duration_options(mut self, duration: DurationConversionOptions) -> Self {
        self.conversion = self.conversion.with_duration_options(duration);
        self
    }

    /// Returns a copy with different numeric conversion options.
    ///
    /// # Parameters
    ///
    /// * `numeric` - New numeric conversion policy and resource limits.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_numeric_options(mut self, numeric: NumericConversionOptions) -> Self {
        self.conversion = self.conversion.with_numeric_options(numeric);
        self
    }
}

impl Default for ReadPolicy {
    /// Creates the default conversion and bounded interpolation policy.
    #[inline]
    fn default() -> Self {
        Self {
            conversion: DataConversionOptions::default(),
            interpolation_sources: InterpolationSources::ConfigOnly,
            max_interpolation_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            max_interpolation_expansions: DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
            max_interpolation_output_bytes: DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
        }
    }
}

impl AsRef<DataConversionOptions> for ReadPolicy {
    /// Borrows the underlying data conversion options.
    #[inline(always)]
    fn as_ref(&self) -> &DataConversionOptions {
        &self.conversion
    }
}

impl From<DataConversionOptions> for ReadPolicy {
    /// Creates a read policy from data conversion options.
    #[inline]
    fn from(conversion: DataConversionOptions) -> Self {
        Self {
            conversion,
            ..Self::default()
        }
    }
}
