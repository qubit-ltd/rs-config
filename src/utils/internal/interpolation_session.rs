// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;

use super::InterpolationResource;
use crate::ConfigError;
use crate::ConfigResult;
use crate::options::ReadPolicy;

/// Resource limits shared by every scalar expanded during one logical read.
pub(crate) struct InterpolationSession {
    /// Placeholder expansions accumulated across one logical read.
    expansions: ResourceBudget<InterpolationResource, usize>,
    /// A point limit checked for each intermediate or final expanded value;
    /// output sizes are not accumulated across different values here.
    output: ResourceLimit<InterpolationResource, usize>,
    /// Expansion limit retained for error diagnostics.
    max_expansions: usize,
    /// Output-byte limit retained for diagnostics and caller admission checks.
    max_output_bytes: usize,
}

impl InterpolationSession {
    /// Creates a session using the limits from one read policy.
    /// # Parameters
    ///
    /// * `options` - Options that control source key selection and
    ///   normalization.
    pub(crate) fn new(options: &ReadPolicy) -> Self {
        Self {
            expansions: ResourceBudget::new(
                InterpolationResource::Expansions,
                options.max_interpolation_expansions(),
            ),
            output: ResourceLimit::new(
                InterpolationResource::OutputBytes,
                options.max_interpolation_output_bytes(),
            ),
            max_expansions: options.max_interpolation_expansions(),
            max_output_bytes: options.max_interpolation_output_bytes(),
        }
    }

    /// Charges one parsed placeholder to this read.
    /// # Parameters
    ///
    /// * `path` - Configuration path or filesystem path used by this operation.
    pub(crate) fn consume_expansion(&mut self, path: &str) -> ConfigResult<()> {
        self.expansions
            .try_consume(1)
            .map_err(|_| ConfigError::SubstitutionExpansionLimitExceeded {
                path: path.to_string(),
                max_expansions: self.max_expansions,
            })
    }

    /// Checks an intermediate or final expanded value against its byte limit.
    /// # Parameters
    ///
    /// * `bytes` - Number of output bytes to account for.
    /// * `path` - Configuration path or filesystem path used by this operation.
    pub(crate) fn check_output(&self, bytes: usize, path: &str) -> ConfigResult<()> {
        self.output
            .check(bytes)
            .map_err(|_| ConfigError::SubstitutionOutputTooLarge {
                path: path.to_string(),
                max_output_bytes: self.max_output_bytes,
            })
    }

    /// Returns the byte limit for each expanded value.
    #[inline]
    pub(crate) const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }
}
