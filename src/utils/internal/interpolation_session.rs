// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;

use super::InterpolationResource;
use crate::ConfigError;
use crate::ConfigResult;
use crate::options::ReadPolicy;

/// Resource limits shared by every scalar expanded during one logical read.
pub(crate) struct InterpolationSession {
    expansions: ResourceBudget<InterpolationResource, usize>,
    output: ResourceLimit<InterpolationResource, usize>,
    max_expansions: usize,
    max_output_bytes: usize,
}

impl InterpolationSession {
    /// Creates a session using the limits from one read policy.
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
    pub(crate) fn consume_expansion(&mut self, path: &str) -> ConfigResult<()> {
        self.expansions.try_consume(1).map_err(|_| {
            ConfigError::SubstitutionExpansionLimitExceeded {
                path: path.to_string(),
                max_expansions: self.max_expansions,
            }
        })
    }

    /// Checks an intermediate or final expanded value against its byte limit.
    pub(crate) fn check_output(&self, bytes: usize, path: &str) -> ConfigResult<()> {
        self.output
            .check(bytes)
            .map_err(|_| ConfigError::SubstitutionOutputTooLarge {
                path: path.to_string(),
                max_output_bytes: self.max_output_bytes,
            })
    }

    pub(crate) const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }
}
