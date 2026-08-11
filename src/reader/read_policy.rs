// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow multiple-public-types

use qubit_datatype::BlankStringPolicy;
use qubit_datatype::BooleanConversionPolicy;
use qubit_datatype::CollectionConversionLimits;
use qubit_datatype::CollectionConversionPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionPolicy;
use qubit_datatype::DurationConversionLimits;
use qubit_datatype::DurationConversionPolicy;
use qubit_datatype::EmptyItemPolicy;
use qubit_datatype::NumericConversionLimits;
use qubit_datatype::NumericConversionPolicy;
use qubit_datatype::StringConversionPolicy;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::DEFAULT_MAX_SUBSTITUTION_DEPTH;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_EXPANSIONS;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES;

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
    /// Semantic rules used by typed value conversions.
    conversion_policy: ConversionPolicy,
    /// Resource limits applied independently to each ordinary read.
    conversion_limits: ConversionLimits,
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
            conversion_policy: ConversionPolicy::env_friendly(),
            ..Self::default()
        }
    }

    /// Gets the semantic data conversion policy.
    ///
    /// # Returns
    ///
    /// Policy used by the shared `qubit-datatype` conversion layer.
    #[inline(always)]
    pub const fn conversion_policy(&self) -> &ConversionPolicy {
        &self.conversion_policy
    }

    /// Gets the resource limits for one logical conversion operation.
    ///
    /// # Returns
    ///
    /// Limits used by the shared `qubit-datatype` conversion layer.
    #[inline(always)]
    pub const fn conversion_limits(&self) -> &ConversionLimits {
        &self.conversion_limits
    }

    /// Returns a copy with a different conversion policy.
    #[inline(always)]
    pub fn with_conversion_policy(mut self, policy: ConversionPolicy) -> Self {
        self.conversion_policy = policy;
        self
    }

    /// Returns a copy with different conversion resource limits.
    #[inline(always)]
    pub fn with_conversion_limits(mut self, limits: ConversionLimits) -> Self {
        self.conversion_limits = limits;
        self
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
    pub const fn with_interpolation_sources(
        mut self,
        sources: InterpolationSources,
    ) -> Self {
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
    /// Updated policy.
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
    /// Updated policy.
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
    /// Updated policy.
    pub fn with_blank_string_policy(
        mut self,
        policy: BlankStringPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_blank_string_policy(policy);
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
        self.conversion_policy =
            self.conversion_policy.with_empty_item_policy(policy);
        self
    }

    /// Returns a copy with a different string conversion policy.
    ///
    /// # Parameters
    ///
    /// * `string` - New string conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_string_policy(
        mut self,
        string: StringConversionPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_string_policy(string);
        self
    }

    /// Returns a copy with a different Boolean conversion policy.
    ///
    /// # Parameters
    ///
    /// * `boolean` - New boolean conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_boolean_policy(
        mut self,
        boolean: BooleanConversionPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_boolean_policy(boolean);
        self
    }

    /// Returns a copy with a different collection conversion policy.
    ///
    /// # Parameters
    ///
    /// * `collection` - New collection conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_collection_policy(
        mut self,
        collection: CollectionConversionPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_collection_policy(collection);
        self
    }

    /// Returns a copy with different collection conversion limits.
    pub fn with_collection_limits(
        mut self,
        collection: CollectionConversionLimits,
    ) -> Self {
        self.conversion_limits =
            self.conversion_limits.with_collection_limits(collection);
        self
    }

    /// Returns a copy with a different duration conversion policy.
    ///
    /// # Parameters
    ///
    /// * `duration` - New duration conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_duration_policy(
        mut self,
        duration: DurationConversionPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_duration_policy(duration);
        self
    }

    /// Returns a copy with different duration conversion limits.
    pub fn with_duration_limits(
        mut self,
        duration: DurationConversionLimits,
    ) -> Self {
        self.conversion_limits =
            self.conversion_limits.with_duration_limits(duration);
        self
    }

    /// Returns a copy with a different numeric conversion policy.
    ///
    /// # Parameters
    ///
    /// * `numeric` - New numeric conversion policy.
    ///
    /// # Returns
    ///
    /// Updated policy.
    pub fn with_numeric_policy(
        mut self,
        numeric: NumericConversionPolicy,
    ) -> Self {
        self.conversion_policy =
            self.conversion_policy.with_numeric_policy(numeric);
        self
    }

    /// Returns a copy with different numeric conversion limits.
    pub fn with_numeric_limits(
        mut self,
        numeric: NumericConversionLimits,
    ) -> Self {
        self.conversion_limits =
            self.conversion_limits.with_numeric_limits(numeric);
        self
    }
}

impl Default for ReadPolicy {
    /// Creates the default conversion and bounded interpolation policy.
    #[inline]
    fn default() -> Self {
        Self {
            conversion_policy: ConversionPolicy::default(),
            conversion_limits: ConversionLimits::default(),
            interpolation_sources: InterpolationSources::ConfigOnly,
            max_interpolation_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            max_interpolation_expansions: DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
            max_interpolation_output_bytes:
                DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
        }
    }
}

impl AsRef<ConversionPolicy> for ReadPolicy {
    /// Borrows the underlying data conversion policy.
    #[inline(always)]
    fn as_ref(&self) -> &ConversionPolicy {
        &self.conversion_policy
    }
}

impl AsRef<ConversionLimits> for ReadPolicy {
    /// Borrows the underlying data conversion limits.
    #[inline(always)]
    fn as_ref(&self) -> &ConversionLimits {
        &self.conversion_limits
    }
}

impl From<ConversionPolicy> for ReadPolicy {
    /// Creates a read policy from a data conversion policy.
    #[inline]
    fn from(conversion_policy: ConversionPolicy) -> Self {
        Self {
            conversion_policy,
            ..Self::default()
        }
    }
}
