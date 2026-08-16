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
    /// Creates a builder initialized with the default read policy.
    #[inline]
    pub fn builder() -> ReadPolicyBuilder {
        ReadPolicyBuilder::new()
    }

    /// Creates a builder initialized from an existing read policy.
    #[inline]
    pub fn builder_from(policy: &Self) -> ReadPolicyBuilder {
        ReadPolicyBuilder {
            conversion_policy: policy.conversion_policy.clone(),
            conversion_limits: policy.conversion_limits.clone(),
            interpolation_sources: policy.interpolation_sources,
            max_interpolation_depth: policy.max_interpolation_depth,
            max_interpolation_expansions: policy.max_interpolation_expansions,
            max_interpolation_output_bytes: policy.max_interpolation_output_bytes,
        }
    }

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
        Self::builder()
            .conversion_policy(ConversionPolicy::env_friendly())
            .build()
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
}

/// Builder for [`ReadPolicy`].
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPolicyBuilder {
    conversion_policy: ConversionPolicy,
    conversion_limits: ConversionLimits,
    interpolation_sources: InterpolationSources,
    max_interpolation_depth: usize,
    max_interpolation_expansions: usize,
    max_interpolation_output_bytes: usize,
}

impl ReadPolicyBuilder {
    fn rebuild_conversion_policy(&self) -> ConversionPolicy {
        ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build()
    }

    fn rebuild_conversion_limits(&self) -> ConversionLimits {
        ConversionLimits::builder()
            .numeric_limits(*self.conversion_limits.numeric())
            .collection_limits(*self.conversion_limits.collection())
            .duration_limits(*self.conversion_limits.duration())
            .structured_limits(*self.conversion_limits.structured())
            .operation_limits(*self.conversion_limits.operation())
            .build()
    }

    /// Creates a builder initialized with the default read policy.
    pub fn new() -> Self {
        Self {
            conversion_policy: ConversionPolicy::default(),
            conversion_limits: ConversionLimits::default(),
            interpolation_sources: InterpolationSources::ConfigOnly,
            max_interpolation_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            max_interpolation_expansions: DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
            max_interpolation_output_bytes: DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
        }
    }

    /// Sets the data conversion policy.
    pub fn conversion_policy(mut self, policy: ConversionPolicy) -> Self {
        self.conversion_policy = policy;
        self
    }

    /// Sets the data conversion limits.
    pub fn conversion_limits(mut self, limits: ConversionLimits) -> Self {
        self.conversion_limits = limits;
        self
    }

    /// Sets the interpolation sources.
    pub const fn interpolation_sources(mut self, sources: InterpolationSources) -> Self {
        self.interpolation_sources = sources;
        self
    }

    /// Sets the maximum recursive interpolation depth.
    pub const fn max_interpolation_depth(mut self, maximum: usize) -> Self {
        self.max_interpolation_depth = maximum;
        self
    }

    /// Sets the maximum placeholder-resolution count per read.
    pub const fn max_interpolation_expansions(mut self, maximum: usize) -> Self {
        self.max_interpolation_expansions = maximum;
        self
    }

    /// Sets the maximum interpolated output byte length.
    pub const fn max_interpolation_output_bytes(mut self, maximum: usize) -> Self {
        self.max_interpolation_output_bytes = maximum;
        self
    }

    /// Sets the blank string conversion policy.
    pub fn blank_string_policy(mut self, policy: BlankStringPolicy) -> Self {
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(policy)
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the empty collection item policy.
    pub fn empty_item_policy(mut self, policy: EmptyItemPolicy) -> Self {
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(policy)
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the string conversion policy.
    pub fn string_policy(mut self, policy: StringConversionPolicy) -> Self {
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(policy)
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the Boolean conversion policy.
    pub fn boolean_policy(mut self, policy: BooleanConversionPolicy) -> Self {
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(policy)
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the collection conversion policy.
    pub fn collection_policy(mut self, policy: CollectionConversionPolicy) -> Self {
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(policy)
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the collection conversion limits.
    pub fn collection_limits(mut self, limits: CollectionConversionLimits) -> Self {
        self.conversion_limits = self.rebuild_conversion_limits();
        self.conversion_limits = ConversionLimits::builder()
            .numeric_limits(*self.conversion_limits.numeric())
            .collection_limits(limits)
            .duration_limits(*self.conversion_limits.duration())
            .structured_limits(*self.conversion_limits.structured())
            .operation_limits(*self.conversion_limits.operation())
            .build();
        self
    }

    /// Sets the duration conversion policy.
    pub fn duration_policy(mut self, policy: DurationConversionPolicy) -> Self {
        self.conversion_policy = self.rebuild_conversion_policy();
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(self.conversion_policy.numeric().clone())
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(policy)
            .build();
        self
    }

    /// Sets the duration conversion limits.
    pub fn duration_limits(mut self, limits: DurationConversionLimits) -> Self {
        self.conversion_limits = ConversionLimits::builder()
            .numeric_limits(*self.conversion_limits.numeric())
            .collection_limits(*self.conversion_limits.collection())
            .duration_limits(limits)
            .structured_limits(*self.conversion_limits.structured())
            .operation_limits(*self.conversion_limits.operation())
            .build();
        self
    }

    /// Sets the numeric conversion policy.
    pub fn numeric_policy(mut self, policy: NumericConversionPolicy) -> Self {
        self.conversion_policy = self.rebuild_conversion_policy();
        self.conversion_policy = ConversionPolicy::builder()
            .numeric_policy(policy)
            .string_policy(self.conversion_policy.string().clone())
            .blank_string_policy(self.conversion_policy.string().blank_string_policy())
            .boolean_policy(self.conversion_policy.boolean().clone())
            .collection_policy(self.conversion_policy.collection().clone())
            .empty_item_policy(self.conversion_policy.collection().empty_item_policy())
            .duration_policy(self.conversion_policy.duration().clone())
            .build();
        self
    }

    /// Sets the numeric conversion limits.
    pub fn numeric_limits(mut self, limits: NumericConversionLimits) -> Self {
        self.conversion_limits = ConversionLimits::builder()
            .numeric_limits(limits)
            .collection_limits(*self.conversion_limits.collection())
            .duration_limits(*self.conversion_limits.duration())
            .structured_limits(*self.conversion_limits.structured())
            .operation_limits(*self.conversion_limits.operation())
            .build();
        self
    }

    /// Builds the configured read policy.
    pub fn build(self) -> ReadPolicy {
        ReadPolicy {
            conversion_policy: self.conversion_policy,
            conversion_limits: self.conversion_limits,
            interpolation_sources: self.interpolation_sources,
            max_interpolation_depth: self.max_interpolation_depth,
            max_interpolation_expansions: self.max_interpolation_expansions,
            max_interpolation_output_bytes: self.max_interpolation_output_bytes,
        }
    }
}

impl Default for ReadPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ReadPolicy {
    /// Creates the default conversion and bounded interpolation policy.
    #[inline]
    fn default() -> Self {
        Self::builder().build()
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
        Self::builder().conversion_policy(conversion_policy).build()
    }
}
