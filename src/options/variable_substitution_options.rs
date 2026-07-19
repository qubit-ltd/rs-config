// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde::{
    Deserialize,
    Serialize,
};

use crate::constants::{
    DEFAULT_MAX_SUBSTITUTION_DEPTH,
    DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
    DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
};

/// Controls variable substitution and bounds the resources used by expansion.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VariableSubstitutionOptions {
    /// Whether `${...}` placeholders are expanded during typed reads.
    enabled: bool,
    /// Whether unresolved names may fall back to process environment
    /// variables.
    environment_fallback_enabled: bool,
    /// Maximum active variable-reference chain length.
    max_depth: usize,
    /// Maximum number of placeholder resolutions in one read.
    max_expansions: usize,
    /// Maximum UTF-8 byte length of one expanded value.
    max_output_bytes: usize,
}

impl VariableSubstitutionOptions {
    /// Returns whether variable substitution is enabled.
    ///
    /// # Returns
    ///
    /// `true` when typed reads expand `${...}` placeholders.
    #[inline(always)]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns whether unresolved variables may use the process environment.
    ///
    /// # Returns
    ///
    /// `true` when environment fallback is enabled.
    #[inline(always)]
    pub const fn is_environment_fallback_enabled(&self) -> bool {
        self.environment_fallback_enabled
    }

    /// Returns the maximum recursive substitution depth.
    ///
    /// # Returns
    ///
    /// The maximum active variable-reference chain length.
    #[inline(always)]
    pub const fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Returns the maximum placeholder-resolution count per read.
    ///
    /// # Returns
    ///
    /// The configured expansion-count limit.
    #[inline(always)]
    pub const fn max_expansions(&self) -> usize {
        self.max_expansions
    }

    /// Returns the maximum expanded UTF-8 byte length.
    ///
    /// # Returns
    ///
    /// The configured output-size limit in bytes.
    #[inline(always)]
    pub const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }

    /// Returns a copy with variable substitution enabled or disabled.
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether typed reads expand placeholders.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns a copy with environment fallback enabled or disabled.
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether unresolved names may use process environment
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

    /// Returns a copy with a different recursive-depth limit.
    ///
    /// # Parameters
    ///
    /// * `max_depth` - Maximum active variable-reference chain length.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Returns a copy with a different placeholder-resolution limit.
    ///
    /// # Parameters
    ///
    /// * `max_expansions` - Maximum number of resolved placeholders per read.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_expansions(mut self, max_expansions: usize) -> Self {
        self.max_expansions = max_expansions;
        self
    }

    /// Returns a copy with a different expanded-output byte limit.
    ///
    /// # Parameters
    ///
    /// * `max_output_bytes` - Maximum UTF-8 bytes in one expanded value.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[inline(always)]
    pub const fn with_max_output_bytes(
        mut self,
        max_output_bytes: usize,
    ) -> Self {
        self.max_output_bytes = max_output_bytes;
        self
    }
}

impl Default for VariableSubstitutionOptions {
    /// Creates the bounded default substitution policy.
    #[inline]
    fn default() -> Self {
        Self {
            enabled: true,
            environment_fallback_enabled: false,
            max_depth: DEFAULT_MAX_SUBSTITUTION_DEPTH,
            max_expansions: DEFAULT_MAX_SUBSTITUTION_EXPANSIONS,
            max_output_bytes: DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES,
        }
    }
}
