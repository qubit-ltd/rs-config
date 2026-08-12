// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Shared local and aggregate accounting for configuration source loading.

use qubit_budget::BudgetError;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;

use super::SourceLimitKind;
use super::SourceLimits;
use crate::ConfigError;
use crate::ConfigResult;

/// Owned resource budgets for one source or composite scope.
struct SourceLoadBudget {
    input_bytes: ResourceBudget<SourceLimitKind, usize>,
    properties: ResourceBudget<SourceLimitKind, usize>,
    nodes: ResourceBudget<SourceLimitKind, usize>,
    sources: ResourceBudget<SourceLimitKind, usize>,
    depth: ResourceLimit<SourceLimitKind, usize>,
}

impl SourceLoadBudget {
    /// Creates an unused budget from one source policy.
    fn new(limits: SourceLimits) -> Self {
        Self {
            input_bytes: ResourceBudget::new(SourceLimitKind::InputBytes, limits.max_input_bytes()),
            properties: ResourceBudget::new(
                SourceLimitKind::PropertyCount,
                limits.max_properties(),
            ),
            nodes: ResourceBudget::new(SourceLimitKind::NodeCount, limits.max_nodes()),
            sources: ResourceBudget::new(SourceLimitKind::SourceCount, limits.max_sources()),
            depth: ResourceLimit::new(SourceLimitKind::NestingDepth, limits.max_nesting_depth()),
        }
    }
}

/// Budget session shared by one source and all of its aggregate ancestors.
///
/// A child session owns its local budget and temporarily borrows every parent
/// budget. Each cumulative charge is checked against all scopes before any
/// scope is changed.
pub struct SourceLoadSession<'a> {
    source_id: String,
    local: SourceLoadBudget,
    ancestor_ids: Vec<String>,
    ancestors: Vec<&'a mut SourceLoadBudget>,
}

impl SourceLoadSession<'static> {
    /// Creates a root source-loading session.
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
    /// Creates a child session that also charges every budget in this session.
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

    /// Returns the stable identifier of the source currently being loaded.
    #[inline(always)]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Charges raw input bytes to every active budget scope.
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        let source_id = self.source_id.clone();
        let ancestor_ids = self.ancestor_ids.clone();
        let mut budgets = self
            .ancestors
            .iter_mut()
            .map(|budget| &mut budget.input_bytes)
            .collect::<Vec<_>>();
        budgets.push(&mut self.local.input_bytes);
        Self::consume_group(&source_id, &ancestor_ids, budgets, amount)
    }

    /// Charges emitted properties to every active budget scope.
    pub fn consume_properties(&mut self, amount: usize) -> ConfigResult<()> {
        let source_id = self.source_id.clone();
        let ancestor_ids = self.ancestor_ids.clone();
        let mut budgets = self
            .ancestors
            .iter_mut()
            .map(|budget| &mut budget.properties)
            .collect::<Vec<_>>();
        budgets.push(&mut self.local.properties);
        Self::consume_group(&source_id, &ancestor_ids, budgets, amount)
    }

    /// Charges parsed structural nodes to every active budget scope.
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        let source_id = self.source_id.clone();
        let ancestor_ids = self.ancestor_ids.clone();
        let mut budgets = self
            .ancestors
            .iter_mut()
            .map(|budget| &mut budget.nodes)
            .collect::<Vec<_>>();
        budgets.push(&mut self.local.nodes);
        Self::consume_group(&source_id, &ancestor_ids, budgets, amount)
    }

    /// Charges admitted child sources to every active budget scope.
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        let source_id = self.source_id.clone();
        let ancestor_ids = self.ancestor_ids.clone();
        let mut budgets = self
            .ancestors
            .iter_mut()
            .map(|budget| &mut budget.sources)
            .collect::<Vec<_>>();
        budgets.push(&mut self.local.sources);
        Self::consume_group(&source_id, &ancestor_ids, budgets, amount)
    }

    /// Checks a root-relative depth against every active budget scope.
    pub fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            budget
                .depth
                .check(depth)
                .map_err(|source| self.limit_error(self.ancestor_ids[index].clone(), source))?;
        }
        self.local
            .depth
            .check(depth)
            .map_err(|source| self.limit_error(self.source_id.clone(), source))
    }

    /// Atomically consumes one cumulative resource across all scopes.
    fn consume_group(
        source_id: &str,
        ancestor_ids: &[String],
        mut budgets: Vec<&mut ResourceBudget<SourceLimitKind, usize>>,
        amount: usize,
    ) -> ConfigResult<()> {
        ResourceBudget::try_consume_group(&mut budgets, amount).map_err(|error| {
            let index = error.index();
            let budget_id = ancestor_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| source_id.to_string());
            ConfigError::SourceLimitExceeded {
                source_id: source_id.to_string(),
                budget_id,
                kind: *error.source_error().resource(),
                limit: error
                    .source_error()
                    .limit()
                    .expect("cumulative budget errors always carry a limit"),
                observed_at_least: error
                    .source_error()
                    .limit()
                    .expect("cumulative budget errors always carry a limit")
                    .saturating_sub(
                        error
                            .source_error()
                            .remaining()
                            .expect("cumulative budget errors always carry remaining capacity"),
                    )
                    .saturating_add(
                        error
                            .source_error()
                            .requested()
                            .expect("cumulative budget errors always carry a request"),
                    ),
                source: error.into_source_error(),
            }
        })
    }

    /// Wraps a point-limit failure with source and budget scope context.
    fn limit_error(
        &self,
        budget_id: String,
        source: BudgetError<SourceLimitKind, usize>,
    ) -> ConfigError {
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.clone(),
            budget_id,
            kind: *source.resource(),
            limit: source
                .maximum()
                .expect("point-limit errors always carry a maximum"),
            observed_at_least: source
                .observed_lower_bound()
                .expect("point-limit errors always carry an observation"),
            source,
        }
    }
}
