// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Mutable accounting for configuration source ingestion.

use crate::{
    ConfigError,
    ConfigResult,
    SourceLimitKind,
};

use super::SourceLimits;

/// Tracks resource use while one source is parsed and flattened.
pub(crate) struct SourceBudget<'a> {
    source_id: &'a str,
    limits: SourceLimits,
    input_bytes: usize,
    properties: usize,
}

impl<'a> SourceBudget<'a> {
    /// Creates an empty budget for one labeled source.
    pub(crate) const fn new(source_id: &'a str, limits: SourceLimits) -> Self {
        Self {
            source_id,
            limits,
            input_bytes: 0,
            properties: 0,
        }
    }

    /// Accounts for input bytes.
    pub(crate) fn consume_input_bytes(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        self.input_bytes = self.consume(
            SourceLimitKind::InputBytes,
            self.input_bytes,
            amount,
            self.limits.max_input_bytes(),
        )?;
        Ok(())
    }

    /// Accounts for emitted assignments.
    pub(crate) fn consume_properties(
        &mut self,
        amount: usize,
    ) -> ConfigResult<()> {
        self.properties = self.consume(
            SourceLimitKind::PropertyCount,
            self.properties,
            amount,
            self.limits.max_properties(),
        )?;
        Ok(())
    }

    /// Checks a root-relative nesting depth without accumulating it.
    pub(crate) fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        let limit = self.limits.max_nesting_depth();
        if depth > limit {
            Err(self.limit_error(SourceLimitKind::NestingDepth, limit, depth))
        } else {
            Ok(())
        }
    }

    /// Adds resource usage and returns a structured limit error on overflow.
    fn consume(
        &self,
        kind: SourceLimitKind,
        current: usize,
        amount: usize,
        limit: usize,
    ) -> ConfigResult<usize> {
        let observed = current.saturating_add(amount);
        if observed > limit {
            Err(self.limit_error(kind, limit, observed))
        } else {
            Ok(observed)
        }
    }

    /// Creates a source limit error.
    fn limit_error(
        &self,
        kind: SourceLimitKind,
        limit: usize,
        observed_at_least: usize,
    ) -> ConfigError {
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.to_string(),
            kind,
            limit,
            observed_at_least,
        }
    }
}
