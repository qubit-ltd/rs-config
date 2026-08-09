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
use qubit_budget::LimitExceeded;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;

use super::SourceLimits;
use crate::ConfigError;
use crate::ConfigResult;
use crate::SourceLimitKind;

/// Tracks resource use while one source is parsed and flattened.
pub(crate) struct SourceBudget<'a> {
    source_id: &'a str,
    input_bytes: ResourceBudget<SourceLimitKind>,
    properties: ResourceBudget<SourceLimitKind>,
    nesting_depth: ResourceLimit<SourceLimitKind>,
}

impl<'a> SourceBudget<'a> {
    /// Creates an empty budget for one labeled source.
    pub(crate) const fn new(source_id: &'a str, limits: SourceLimits) -> Self {
        Self {
            source_id,
            input_bytes: ResourceBudget::new(ResourceLimit::bounded(
                SourceLimitKind::InputBytes,
                limits.max_input_bytes(),
            )),
            properties: ResourceBudget::new(ResourceLimit::bounded(
                SourceLimitKind::PropertyCount,
                limits.max_properties(),
            )),
            nesting_depth: ResourceLimit::bounded(
                SourceLimitKind::NestingDepth,
                limits.max_nesting_depth(),
            ),
        }
    }

    /// Accounts for input bytes.
    pub(crate) fn consume_input_bytes(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.input_bytes.try_charge(amount);
        result.map_err(|error| self.limit_error(error))
    }

    /// Accounts for emitted assignments.
    pub(crate) fn consume_properties(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.properties.try_charge(amount);
        result.map_err(|error| self.limit_error(error))
    }

    /// Checks a root-relative nesting depth without accumulating it.
    pub(crate) fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        self.nesting_depth
            .check(depth)
            .map_err(|error| self.point_limit_error(error))
    }

    /// Creates a source limit error.
    fn limit_error(
        &self,
        error: BudgetError<SourceLimitKind, usize>,
    ) -> ConfigError {
        match error {
            BudgetError::Exceeded {
                kind,
                maximum,
                observed,
                ..
            } => ConfigError::SourceLimitExceeded {
                source_id: self.source_id.to_string(),
                kind,
                limit: maximum,
                observed_at_least: observed,
            },
            BudgetError::CounterOverflow { kind, .. } => {
                ConfigError::SourceLimitExceeded {
                    source_id: self.source_id.to_string(),
                    limit: usize::MAX,
                    kind,
                    observed_at_least: usize::MAX,
                }
            }
            BudgetError::Closed { .. } => {
                unreachable!("SourceBudget never closes its resource budgets")
            }
        }
    }

    fn point_limit_error(
        &self,
        error: LimitExceeded<SourceLimitKind>,
    ) -> ConfigError {
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.to_string(),
            kind: error.into_kind(),
            limit: error.maximum(),
            observed_at_least: error.observed(),
        }
    }
}
