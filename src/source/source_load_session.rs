// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared local and aggregate accounting for configuration source loading.

use qubit_budget::BudgetError;
use qubit_budget::InsufficientBudgetError;

use super::SourceLimitKind;
use super::SourceLimits;
use super::internal::cumulative_dimension::CumulativeDimension;
use super::internal::source_load_budget::SourceLoadBudget;
use crate::ConfigError;
use crate::ConfigResult;

/// Budget session shared by one source and all of its aggregate ancestors.
///
/// A child session owns its local budget and temporarily borrows every parent
/// budget. Each cumulative charge is checked against all scopes before any
/// scope is changed.
pub(crate) struct SourceLoadSession<'a> {
    source_id: String,
    local: SourceLoadBudget,
    ancestor_ids: Vec<String>,
    ancestors: Vec<&'a mut SourceLoadBudget>,
}
impl SourceLoadSession<'static> {
    /// Creates a root source-loading session.
    #[doc(hidden)]
    pub fn new(source_id: impl Into<String>, limits: SourceLimits) -> Self {
        Self {
            source_id: source_id.into(),
            local: SourceLoadBudget::new(limits),
            ancestor_ids: Vec::new(),
            ancestors: Vec::new(),
        }
    }
}

impl<'a> SourceLoadSession<'a> {
    #[inline]
    pub(in crate::source) fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Creates a child session that also charges every budget in this session.
    #[doc(hidden)]
    pub fn child<'child>(
        &'child mut self,
        source_id: impl Into<String>,
        limits: SourceLimits,
    ) -> SourceLoadSession<'child> {
        let mut ancestors = self
            .ancestors
            .iter_mut()
            .map(|budget| &mut **budget)
            .collect::<Vec<_>>();
        ancestors.push(&mut self.local);
        let mut ancestor_ids = self.ancestor_ids.clone();
        ancestor_ids.push(self.source_id.clone());
        SourceLoadSession {
            source_id: source_id.into(),
            local: SourceLoadBudget::new(limits),
            ancestor_ids,
            ancestors,
        }
    }

    /// Charges raw input bytes to every active budget scope.
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::InputBytes, amount)
    }

    /// Charges emitted properties to every active budget scope.
    pub fn consume_properties(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Properties, amount)
    }

    /// Charges parsed structural nodes to every active budget scope.
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Nodes, amount)
    }

    /// Charges admitted child sources to every active budget scope.
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Sources, amount)
    }

    /// Checks a root-relative depth against every active budget scope.
    pub fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            budget
                .depth
                .check(depth)
                .map_err(|source| self.limit_error(self.ancestor_ids[index].clone(), source.into()))?;
        }
        self.local
            .depth
            .check(depth)
            .map_err(|source| self.limit_error(self.source_id.clone(), source.into()))
    }

    /// Atomically consumes one cumulative resource across all active scopes.
    fn consume_cumulative(&mut self, dimension: CumulativeDimension, amount: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            if let Err(source) = dimension.get(budget).check_available(amount) {
                let budget_id = self
                    .ancestor_ids
                    .get(index)
                    .map_or(self.source_id.as_str(), String::as_str);
                return Err(Self::cumulative_limit_error(&self.source_id, budget_id, source));
            }
        }
        if let Err(source) = dimension.get(&self.local).check_available(amount) {
            return Err(Self::cumulative_limit_error(&self.source_id, &self.source_id, source));
        }

        for budget in &mut self.ancestors {
            let _ = dimension.get_mut(budget).consume_available(amount);
        }
        let _ = dimension.get_mut(&mut self.local).consume_available(amount);
        Ok(())
    }

    /// Wraps a cumulative failure with source and budget scope context.
    fn cumulative_limit_error(
        source_id: &str,
        budget_id: &str,
        source: InsufficientBudgetError<SourceLimitKind, usize>,
    ) -> ConfigError {
        ConfigError::SourceLimitExceeded {
            source_id: source_id.to_owned(),
            budget_id: budget_id.to_owned(),
            kind: *source.resource(),
            limit: source.limit(),
            observed_at_least: source.used().saturating_add(source.requested()),
            source: source.into(),
        }
    }

    /// Wraps a point-limit failure with source and budget scope context.
    fn limit_error(&self, budget_id: String, source: BudgetError<SourceLimitKind, usize>) -> ConfigError {
        let limit = source.configured_limit();
        let observed_at_least = match &source {
            BudgetError::LimitExceeded { observed, .. } => observed.lower_bound(),
            BudgetError::Insufficient {
                limit,
                remaining,
                requested,
                ..
            } => (*limit - *remaining).saturating_add(*requested),
        };
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.clone(),
            budget_id,
            kind: *source.resource(),
            limit,
            observed_at_least,
            source,
        }
    }
}
