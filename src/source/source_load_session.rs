// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared local and aggregate accounting for configuration source loading.

use std::sync::Arc;

use qubit_budget::BudgetError;
use qubit_budget::InsufficientBudgetError;
use qubit_budget::ResourceLimit;

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
#[must_use = "use the session to account for source resource usage"]
pub(crate) struct SourceLoadSession<'a> {
    /// Shared diagnostic identity of the source currently being charged.
    /// Child sessions share this text while copying ancestor slots. This may
    /// differ from the ancestor scope that rejects a charge.
    source_id: Arc<str>,
    /// Owned budget for this source's local scope.
    local: SourceLoadBudget,
    /// Shared diagnostic identities corresponding one-to-one with `ancestors`.
    ancestor_ids: Vec<Arc<str>>,
    /// Borrowed ancestor budgets, ordered to match `ancestor_ids`.
    /// A child preserves this order and appends its parent's local budget.
    /// Each cumulative charge checks all scopes before changing any of them.
    ancestors: Vec<&'a mut SourceLoadBudget>,
}
impl SourceLoadSession<'static> {
    /// Creates a root source-loading session.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identifier associated with this source.
    /// * `limits` - Local limits enforced for the source.
    ///
    /// # Returns
    ///
    /// A session with a fresh local budget and no aggregate ancestors.
    #[doc(hidden)]
    pub fn new(source_id: impl Into<String>, limits: SourceLimits) -> Self {
        Self {
            source_id: Arc::<str>::from(source_id.into()),
            local: SourceLoadBudget::new(limits),
            ancestor_ids: Vec::new(),
            ancestors: Vec::new(),
        }
    }
}

impl<'a> SourceLoadSession<'a> {
    /// Creates a child session that also charges every budget in this session.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identifier associated with the child source.
    /// * `limits` - Local limits enforced for the child.
    ///
    /// # Returns
    ///
    /// A child session that charges its local budget and every ancestor budget.
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
        ancestor_ids.push(Arc::clone(&self.source_id));
        SourceLoadSession {
            source_id: Arc::<str>::from(source_id.into()),
            local: SourceLoadBudget::new(limits),
            ancestor_ids,
            ancestors,
        }
    }

    /// Enforces the fixed executor guard for one more composite child edge.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identifier of the child that would exceed the limit.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the next edge is within the executor limit.
    ///
    /// # Errors
    ///
    /// Returns `SourceLimitExceeded` with the root budget scope and rejected
    /// child identifier when the fixed nesting depth would be exceeded.
    pub(crate) fn check_next_source_nesting(&self, source_id: &str) -> ConfigResult<()> {
        const MAX_SOURCE_NESTING: usize = 256;
        let depth = self.ancestor_ids.len().saturating_add(1);
        ResourceLimit::new(SourceLimitKind::SourceNestingDepth, MAX_SOURCE_NESTING)
            .check(depth)
            .map_err(|source| {
                let root_id = self
                    .ancestor_ids
                    .first()
                    .map_or(self.source_id.as_ref(), |id| id.as_ref());
                ConfigError::SourceLimitExceeded {
                    source_id: source_id.to_owned(),
                    budget_id: root_id.to_owned(),
                    kind: SourceLimitKind::SourceNestingDepth,
                    limit: MAX_SOURCE_NESTING,
                    observed_at_least: depth,
                    source: source.into(),
                }
            })
    }

    /// Checks a root-relative depth against every active budget scope.
    ///
    /// # Parameters
    ///
    /// * `depth` - Semantic nesting depth to check.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the depth is within every active limit.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when any active depth limit is exceeded.
    pub fn check_depth(&self, depth: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            budget
                .depth
                .check(depth)
                .map_err(|source| self.limit_error(self.ancestor_ids[index].as_ref(), source.into()))?;
        }
        self.local
            .depth
            .check(depth)
            .map_err(|source| self.limit_error(self.source_id.as_ref(), source.into()))
    }

    /// Checks one input's parser depth against each ancestor and this source.
    ///
    /// # Parameters
    ///
    /// * `depth` - Parser nesting depth for the current input, with root depth
    ///   zero.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the depth is within every active limit.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when any active parser-depth limit is exceeded.
    pub fn check_parser_depth(&self, depth: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            budget
                .parser_depth
                .check(depth)
                .map_err(|source| self.limit_error(self.ancestor_ids[index].as_ref(), source.into()))?;
        }
        self.local
            .parser_depth
            .check(depth)
            .map_err(|source| self.limit_error(self.source_id.as_ref(), source.into()))
    }

    /// Charges raw input bytes to every active budget scope.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of raw input bytes to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active budget accepts the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when an active input-byte limit is exceeded.
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::InputBytes, amount)
    }

    /// Charges emitted properties to every active budget scope.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of property assignments to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active budget accepts the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when an active property limit is exceeded.
    pub fn consume_properties(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Properties, amount)
    }

    /// Charges parsed structural nodes to every active budget scope.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of parsed nodes to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active budget accepts the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when an active node limit is exceeded.
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Nodes, amount)
    }

    /// Charges parser tokens atomically to this source and all aggregate
    /// ancestors.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of parser tokens to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active budget accepts the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when an active parser-token limit is exceeded.
    pub fn consume_parser_tokens(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::ParserTokens, amount)
    }

    /// Charges admitted child sources to every active budget scope.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of child sources to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active budget accepts the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] with the rejecting scope
    /// when an active source-count limit is exceeded.
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        self.consume_cumulative(CumulativeDimension::Sources, amount)
    }

    /// Borrows the identity of the charged source, not necessarily the
    /// identity of the budget scope that rejected a charge.
    ///
    /// # Returns
    ///
    /// The identifier attached to errors originating from this source.
    #[inline]
    pub(in crate::source) fn source_id(&self) -> &str {
        self.source_id.as_ref()
    }

    /// Atomically consumes one cumulative resource across all active scopes.
    ///
    /// # Parameters
    ///
    /// * `dimension` - Cumulative resource dimension to charge.
    /// * `amount` - Requested increment in units of that resource dimension.
    ///
    /// # Returns
    ///
    /// `Ok(())` when every active scope accepts and records the charge.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::SourceLimitExceeded`] without changing any
    /// scope when one active budget rejects the charge.
    fn consume_cumulative(&mut self, dimension: CumulativeDimension, amount: usize) -> ConfigResult<()> {
        for (index, budget) in self.ancestors.iter().enumerate() {
            if let Err(source) = dimension.get(budget).check_available(amount) {
                let budget_id = self
                    .ancestor_ids
                    .get(index)
                    .map_or(self.source_id.as_ref(), Arc::as_ref);
                return Err(Self::cumulative_limit_error(self.source_id.as_ref(), budget_id, source));
            }
        }
        if let Err(source) = dimension.get(&self.local).check_available(amount) {
            return Err(Self::cumulative_limit_error(
                self.source_id.as_ref(),
                self.source_id.as_ref(),
                source,
            ));
        }

        for budget in &mut self.ancestors {
            let _ = dimension.get_mut(budget).consume_available(amount);
        }
        let _ = dimension.get_mut(&mut self.local).consume_available(amount);
        Ok(())
    }

    /// Wraps a cumulative failure with source and budget scope context.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identity of the source whose charge was rejected.
    /// * `budget_id` - Identity of the budget scope that rejected the charge.
    /// * `source` - Underlying budget error with the configured limit, usage,
    ///   and requested amount.
    ///
    /// # Returns
    ///
    /// A source-limit error retaining the rejected source, budget scope, and
    /// observed lower bound.
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
    ///
    /// # Parameters
    ///
    /// * `budget_id` - Identity of the budget scope that rejected the charge.
    /// * `source` - Underlying budget error with the configured limit, usage,
    ///   and requested amount.
    ///
    /// # Returns
    ///
    /// A source-limit error retaining this source, the rejecting scope, and
    /// the observed lower bound.
    fn limit_error(&self, budget_id: &str, source: BudgetError<SourceLimitKind, usize>) -> ConfigError {
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
            source_id: self.source_id.as_ref().to_owned(),
            budget_id: budget_id.to_owned(),
            kind: *source.resource(),
            limit,
            observed_at_least,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceLimits;
    use super::SourceLoadSession;

    #[test]
    fn test_child_shares_ancestor_label_storage() {
        use std::sync::Arc;

        let mut root = SourceLoadSession::new("root-配置源", SourceLimits::default());
        let root_id = Arc::clone(&root.source_id);
        {
            let mut child = root.child("child-配置源", SourceLimits::default());
            assert!(Arc::ptr_eq(&root_id, &child.ancestor_ids[0]));
            let child_id = Arc::clone(&child.source_id);
            let grandchild = child.child("leaf", SourceLimits::default());
            assert!(Arc::ptr_eq(&root_id, &grandchild.ancestor_ids[0]));
            assert!(Arc::ptr_eq(&child_id, &grandchild.ancestor_ids[1]));
        }
        assert_eq!(root.source_id(), "root-配置源");
        root.consume_input_bytes(1).unwrap();
    }
}
