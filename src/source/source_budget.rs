// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Mutable accounting for configuration source ingestion.

use qubit_budget::LimitExceeded;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceBudgetError;
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
    nesting_depth: ResourceLimit,
}

impl<'a> SourceBudget<'a> {
    /// Creates an empty budget for one labeled source.
    pub(crate) fn new(source_id: &'a str, limits: SourceLimits) -> Self {
        Self {
            source_id,
            input_bytes: ResourceBudget::new(
                SourceLimitKind::InputBytes,
                ResourceLimit::new(
                    u64::try_from(limits.max_input_bytes())
                        .expect("usize input limit must fit in u64"),
                ),
            ),
            properties: ResourceBudget::new(
                SourceLimitKind::PropertyCount,
                ResourceLimit::new(
                    u64::try_from(limits.max_properties())
                        .expect("usize property limit must fit in u64"),
                ),
            ),
            nesting_depth: ResourceLimit::new(
                u64::try_from(limits.max_nesting_depth())
                    .expect("usize nesting limit must fit in u64"),
            ),
        }
    }

    /// Accounts for input bytes.
    pub(crate) fn consume_input_bytes(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.input_bytes.try_consume(
            u64::try_from(amount).expect("usize byte count must fit in u64"),
        );
        result.map_err(|error| self.limit_error(error))
    }

    /// Accounts for emitted assignments.
    pub(crate) fn consume_properties(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        let result = self.properties.try_consume(
            u64::try_from(amount)
                .expect("usize property count must fit in u64"),
        );
        result.map_err(|error| self.limit_error(error))
    }

    /// Checks a root-relative nesting depth without accumulating it.
    pub(crate) fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        self.nesting_depth
            .check(
                SourceLimitKind::NestingDepth,
                u64::try_from(depth)
                    .expect("usize nesting depth must fit in u64"),
            )
            .map_err(|error| self.point_limit_error(error))
    }

    /// Creates a source limit error.
    fn limit_error(
        &self,
        error: ResourceBudgetError<SourceLimitKind>,
    ) -> ConfigError {
        let maximum = error.limit().maximum();
        let used = maximum.saturating_sub(error.remaining());
        let observed = used.saturating_add(error.requested());
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.to_string(),
            kind: error.into_resource(),
            limit: usize::try_from(maximum).unwrap_or(usize::MAX),
            observed_at_least: usize::try_from(observed).unwrap_or(usize::MAX),
        }
    }

    fn point_limit_error(
        &self,
        error: LimitExceeded<SourceLimitKind>,
    ) -> ConfigError {
        let limit = error.limit().maximum();
        let observed = error.observed();
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.to_string(),
            kind: error.into_resource(),
            limit: usize::try_from(limit).unwrap_or(usize::MAX),
            observed_at_least: usize::try_from(observed).unwrap_or(usize::MAX),
        }
    }
}
