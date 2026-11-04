// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

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

use super::InterpolationSources;
use super::ReadPolicy;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_DEPTH;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_EXPANSIONS;
use crate::constants::DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES;

/// Builder for [`ReadPolicy`].
///
/// These settings do not make ordinary reads resolve placeholders. The default
/// interpolation source is [`InterpolationSources::ConfigOnly`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::options::InterpolationSources;
/// use qubit_config::ReadPolicy;
///
/// let policy = ReadPolicy::builder()
///     .interpolation_sources(InterpolationSources::ConfigOnly)
///     .max_interpolation_expansions(12)
///     .build();
///
/// assert_eq!(policy.interpolation_sources(), InterpolationSources::ConfigOnly);
/// assert_eq!(policy.max_interpolation_expansions(), 12);
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPolicyBuilder {
    /// Semantic conversion rules used by the completed policy.
    pub(super) conversion_policy: ConversionPolicy,
    /// Resource limits used by the completed policy.
    pub(super) conversion_limits: ConversionLimits,
    /// Interpolation sources used by the completed policy.
    pub(super) interpolation_sources: InterpolationSources,
    /// Maximum active placeholder-reference chain length.
    pub(super) max_interpolation_depth: usize,
    /// Maximum placeholder resolutions per read.
    pub(super) max_interpolation_expansions: usize,
    /// Maximum bytes produced by one interpolation.
    pub(super) max_interpolation_output_bytes: usize,
}

impl ReadPolicyBuilder {
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

    /// Sets the maximum depth of an active placeholder-reference chain.
    ///
    /// The limit counts references followed while resolving a value, not the
    /// number of fields in a read. Expansion uses heap-backed frames instead
    /// of one Rust call frame per reference; active frame storage grows with
    /// the current reference chain. Zero still allows plain
    /// text without placeholders, but rejects a placeholder reference; it
    /// does not mean unlimited depth. Exceeding the limit produces
    /// [`crate::ConfigError::SubstitutionDepthExceeded`]. The default is 64,
    /// as returned by [`ReadPolicy::default`].
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum number of nested placeholder references.
    ///
    /// # Returns
    ///
    /// This builder with the updated depth limit.
    pub const fn max_interpolation_depth(mut self, maximum: usize) -> Self {
        self.max_interpolation_depth = maximum;
        self
    }

    /// Sets the maximum placeholder resolutions in one logical interpolated
    /// read.
    ///
    /// Candidate-key search and structured fields share this count, including
    /// fields that the target visitor later ignores. Zero rejects the first
    /// placeholder; it does not disable the limit or enable interpolation.
    /// Exceeding the limit produces
    /// [`crate::ConfigError::SubstitutionExpansionLimitExceeded`]. The default
    /// is 4,096, as returned by [`ReadPolicy::default`].
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum number of placeholder resolutions admitted per
    ///   read.
    ///
    /// # Returns
    ///
    /// This builder with the updated limit.
    pub const fn max_interpolation_expansions(mut self, maximum: usize) -> Self {
        self.max_interpolation_expansions = maximum;
        self
    }

    /// Sets the maximum UTF-8 byte length of one interpolated value.
    ///
    /// The limit applies independently to each value and to intermediate
    /// results during recursive expansion; it is not a total across all
    /// fields in a read. Zero permits only an empty output and does not disable
    /// the limit. Oversized output produces
    /// [`crate::ConfigError::SubstitutionOutputTooLarge`]. The default is
    /// 1,048,576 bytes, as returned by [`ReadPolicy::default`].
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum UTF-8 byte length for one output or intermediate
    ///   value.
    ///
    /// # Returns
    ///
    /// This builder with the updated output-size limit.
    pub const fn max_interpolation_output_bytes(mut self, maximum: usize) -> Self {
        self.max_interpolation_output_bytes = maximum;
        self
    }

    /// Sets only the blank-string setting of the current string conversion
    /// policy, preserving its other settings. A later
    /// [`string_policy`](Self::string_policy) call replaces the complete
    /// string policy, including this setting.
    ///
    /// # Parameters
    ///
    /// * `policy` - The blank-string handling policy to use.
    ///
    /// # Returns
    ///
    /// This builder with the updated blank-string setting.
    ///
    /// See [`string_policy`](Self::string_policy) for replacing the complete
    /// string conversion policy.
    pub fn blank_string_policy(mut self, policy: BlankStringPolicy) -> Self {
        self.conversion_policy = self
            .conversion_policy
            .into_builder()
            .blank_string_policy(policy)
            .build();
        self
    }

    /// Sets only the empty-item setting of the current collection conversion
    /// policy, preserving its other settings. A later
    /// [`collection_policy`](Self::collection_policy) call replaces the
    /// complete collection policy, including this setting.
    ///
    /// # Parameters
    ///
    /// * `policy` - The policy for handling empty collection items.
    ///
    /// # Returns
    ///
    /// This builder with the updated empty-item setting.
    ///
    /// See [`collection_policy`](Self::collection_policy) for replacing the
    /// complete collection conversion policy.
    pub fn empty_item_policy(mut self, policy: EmptyItemPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().empty_item_policy(policy).build();
        self
    }

    /// Replaces the complete string conversion policy, including its
    /// blank-string setting. A later
    /// [`blank_string_policy`](Self::blank_string_policy) call changes only
    /// that setting.
    ///
    /// # Parameters
    ///
    /// * `policy` - The complete string conversion policy to use.
    ///
    /// # Returns
    ///
    /// This builder with the replacement string conversion policy.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::ReadPolicy;
    /// use qubit_datatype::BlankStringPolicy;
    /// use qubit_datatype::StringConversionPolicy;
    ///
    /// let complete = StringConversionPolicy::builder()
    ///     .blank_string_policy(BlankStringPolicy::Reject)
    ///     .build();
    /// let replaced = ReadPolicy::builder()
    ///     .blank_string_policy(BlankStringPolicy::TreatAsMissing)
    ///     .string_policy(complete.clone())
    ///     .build();
    /// assert_eq!(replaced.conversion_policy().string(), &complete);
    ///
    /// let adjusted = ReadPolicy::builder()
    ///     .string_policy(complete)
    ///     .blank_string_policy(BlankStringPolicy::TreatAsMissing)
    ///     .build();
    /// assert_eq!(
    ///     adjusted.conversion_policy().string().blank_string_policy(),
    ///     BlankStringPolicy::TreatAsMissing,
    /// );
    /// ```
    ///
    /// See [`blank_string_policy`](Self::blank_string_policy) for changing only
    /// the blank-string setting.
    pub fn string_policy(mut self, policy: StringConversionPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().string_policy(policy).build();
        self
    }

    /// Sets the Boolean conversion policy.
    pub fn boolean_policy(mut self, policy: BooleanConversionPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().boolean_policy(policy).build();
        self
    }

    /// Replaces the complete collection conversion policy, including its
    /// empty-item setting. A later
    /// [`empty_item_policy`](Self::empty_item_policy) call changes only that
    /// setting.
    ///
    /// # Parameters
    ///
    /// * `policy` - The complete collection conversion policy to use.
    ///
    /// # Returns
    ///
    /// This builder with the replacement collection conversion policy.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::ReadPolicy;
    /// use qubit_datatype::CollectionConversionPolicy;
    /// use qubit_datatype::EmptyItemPolicy;
    ///
    /// let complete = CollectionConversionPolicy::builder()
    ///     .empty_item_policy(EmptyItemPolicy::Reject)
    ///     .build();
    /// let replaced = ReadPolicy::builder()
    ///     .empty_item_policy(EmptyItemPolicy::Skip)
    ///     .collection_policy(complete.clone())
    ///     .build();
    /// assert_eq!(replaced.conversion_policy().collection(), &complete);
    ///
    /// let adjusted = ReadPolicy::builder()
    ///     .collection_policy(complete)
    ///     .empty_item_policy(EmptyItemPolicy::Skip)
    ///     .build();
    /// assert_eq!(
    ///     adjusted.conversion_policy().collection().empty_item_policy(),
    ///     EmptyItemPolicy::Skip,
    /// );
    /// ```
    ///
    /// See [`empty_item_policy`](Self::empty_item_policy) for changing only the
    /// empty-item setting.
    pub fn collection_policy(mut self, policy: CollectionConversionPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().collection_policy(policy).build();
        self
    }

    /// Sets the collection conversion limits.
    pub fn collection_limits(mut self, limits: CollectionConversionLimits) -> Self {
        self.conversion_limits = self.conversion_limits.into_builder().collection_limits(limits).build();
        self
    }

    /// Sets the duration conversion policy.
    pub fn duration_policy(mut self, policy: DurationConversionPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().duration_policy(policy).build();
        self
    }

    /// Sets the duration conversion limits.
    pub fn duration_limits(mut self, limits: DurationConversionLimits) -> Self {
        self.conversion_limits = self.conversion_limits.into_builder().duration_limits(limits).build();
        self
    }

    /// Sets the numeric conversion policy.
    pub fn numeric_policy(mut self, policy: NumericConversionPolicy) -> Self {
        self.conversion_policy = self.conversion_policy.into_builder().numeric_policy(policy).build();
        self
    }

    /// Sets the numeric conversion limits.
    pub fn numeric_limits(mut self, limits: NumericConversionLimits) -> Self {
        self.conversion_limits = self.conversion_limits.into_builder().numeric_limits(limits).build();
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
