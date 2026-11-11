// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionPolicy;
use serde::Deserialize;
use serde::Serialize;

// Defines policy construction without exposing stored policy fields.
#[path = "read_policy_builder.rs"]
mod read_policy_builder;
pub use read_policy_builder::ReadPolicyBuilder;
// Defines the supported interpolation lookup sources.
#[path = "interpolation_sources.rs"]
mod interpolation_sources;
pub use interpolation_sources::InterpolationSources;

/// Runtime policy that controls configuration conversion and interpolation.
///
/// A policy can be borrowed by [`crate::ConfigReader::read_with`] without
/// changing the configuration's default policy.
///
/// # Examples
///
/// ```rust
/// use qubit_config::options::InterpolationSources;
/// use qubit_config::{Config, ConfigReader, ReadPolicy};
///
/// let mut config = Config::new();
/// config.set("enabled", "true")?;
///
/// let policy = ReadPolicy::builder()
///     .interpolation_sources(InterpolationSources::ConfigThenEnv)
///     .max_interpolation_depth(8)
///     .build();
/// let reader = config.read_with(&policy);
///
/// assert!(reader.get::<bool>("enabled")?);
/// assert_eq!(reader.read_policy().max_interpolation_depth(), 8);
/// assert_eq!(reader.read_policy().interpolation_sources(), InterpolationSources::ConfigThenEnv);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[must_use = "use the returned value"]
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
    /// # Parameters
    ///
    /// * `policy` - Read policy applied while resolving the value.
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
    #[must_use = "use the returned value"]
    pub const fn conversion_policy(&self) -> &ConversionPolicy {
        &self.conversion_policy
    }

    /// Gets the resource limits for one logical conversion operation.
    ///
    /// # Returns
    ///
    /// Limits used by the shared `qubit-datatype` conversion layer.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn conversion_limits(&self) -> &ConversionLimits {
        &self.conversion_limits
    }

    /// Returns the configured interpolation sources.
    ///
    /// # Returns
    ///
    /// The source order used by interpolated reads.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn interpolation_sources(&self) -> InterpolationSources {
        self.interpolation_sources
    }

    /// Returns the maximum recursive interpolation depth.
    ///
    /// # Returns
    ///
    /// The maximum active placeholder-reference chain length.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_interpolation_depth(&self) -> usize {
        self.max_interpolation_depth
    }

    /// Returns the maximum placeholder-resolution count per read.
    ///
    /// # Returns
    ///
    /// The configured interpolation expansion-count limit.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_interpolation_expansions(&self) -> usize {
        self.max_interpolation_expansions
    }

    /// Returns the maximum interpolated UTF-8 byte length.
    ///
    /// # Returns
    ///
    /// The configured output-size limit in bytes.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_interpolation_output_bytes(&self) -> usize {
        self.max_interpolation_output_bytes
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
    /// All other fields use the defaults from [`ReadPolicy::builder`].
    ///
    /// # Parameters
    ///
    /// * `conversion_policy` - The conversion policy retained by the new read
    ///   policy.
    #[inline]
    fn from(conversion_policy: ConversionPolicy) -> Self {
        Self::builder().conversion_policy(conversion_policy).build()
    }
}
