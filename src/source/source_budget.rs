// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Mutable accounting for configuration source ingestion.

use qubit_budget::BudgetError;
use qubit_budget::ResourceBudget;

use super::SourceLimits;
use crate::ConfigError;
use crate::ConfigResult;
use crate::SourceLimitKind;

/// Tracks resource use while one source is parsed and flattened.
pub(crate) struct SourceBudget<'a> {
    source_id: &'a str,
    input_bytes: ResourceBudget<SourceLimitKind, usize>,
    properties: ResourceBudget<SourceLimitKind, usize>,
    nesting_depth: usize,
}

impl<'a> SourceBudget<'a> {
    /// Creates an empty budget for one labeled source.
    pub(crate) fn new(source_id: &'a str, limits: SourceLimits) -> Self {
        Self {
            source_id,
            input_bytes: ResourceBudget::new(
                SourceLimitKind::InputBytes,
                limits.max_input_bytes(),
            ),
            properties: ResourceBudget::new(
                SourceLimitKind::PropertyCount,
                limits.max_properties(),
            ),
            nesting_depth: limits.max_nesting_depth(),
        }
    }

    /// Accounts for input bytes.
    pub(crate) fn consume_input_bytes(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.input_bytes.try_consume(amount);
        result.map_err(|error| self.limit_error(error))
    }

    /// Accounts for emitted assignments.
    pub(crate) fn consume_properties(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.properties.try_consume(amount);
        result.map_err(|error| self.limit_error(error))
    }

    /// Checks a root-relative nesting depth without accumulating it.
    pub(crate) fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        if depth <= self.nesting_depth {
            Ok(())
        } else {
            Err(ConfigError::SourceLimitExceeded {
                source_id: self.source_id.to_string(),
                kind: SourceLimitKind::NestingDepth,
                limit: self.nesting_depth,
                observed_at_least: depth,
            })
        }
    }

    /// Creates a source limit error.
    #[allow(clippy::manual_saturating_arithmetic)]
    fn limit_error(
        &self,
        error: BudgetError<SourceLimitKind, usize>,
    ) -> ConfigError {
        match error {
            BudgetError::Insufficient {
                resource,
                limit,
                remaining,
                requested,
            } => ConfigError::SourceLimitExceeded {
                source_id: self.source_id.to_string(),
                kind: resource,
                limit,
                observed_at_least: (limit - remaining)
                    .checked_add(requested)
                    .unwrap_or(usize::MAX),
            },
            BudgetError::LimitExceeded { .. } => {
                unreachable!("SourceBudget only consumes cumulative resources")
            }
        }
    }
}
