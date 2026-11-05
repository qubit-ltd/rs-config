// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_budget::ResourceBudget;

use crate::source::SourceLimitKind;
use crate::source::internal::source_load_budget::SourceLoadBudget;

/// Selects one cumulative dimension from every active source budget.
#[derive(Clone, Copy)]
pub(in crate::source) enum CumulativeDimension {
    /// Cumulative bytes read from source inputs.
    InputBytes,
    /// Cumulative property assignments across source loads.
    Properties,
    /// Cumulative configuration nodes produced by sources.
    Nodes,
    /// Cumulative parser tokens consumed by source parsers.
    ParserTokens,
    /// Cumulative child sources loaded by composites.
    Sources,
}

impl CumulativeDimension {
    /// Borrows the selected cumulative resource budget.
    ///
    /// # Parameters
    ///
    /// * `budget` - The source-load budgets to query.
    ///
    /// # Returns
    ///
    /// The budget corresponding to this dimension.
    #[must_use = "the selected budget must be checked"]
    #[inline]
    pub(in crate::source) fn get(self, budget: &SourceLoadBudget) -> &ResourceBudget<SourceLimitKind, usize> {
        match self {
            Self::InputBytes => &budget.input_bytes,
            Self::Properties => &budget.properties,
            Self::Nodes => &budget.nodes,
            Self::ParserTokens => &budget.parser_tokens,
            Self::Sources => &budget.sources,
        }
    }

    /// Mutably borrows the selected cumulative resource budget.
    ///
    /// # Parameters
    ///
    /// * `budget` - The source-load budgets to update.
    ///
    /// # Returns
    ///
    /// A mutable reference to the budget corresponding to this dimension.
    #[must_use = "the selected budget must be charged"]
    #[inline]
    pub(in crate::source) fn get_mut(
        self,
        budget: &mut SourceLoadBudget,
    ) -> &mut ResourceBudget<SourceLimitKind, usize> {
        match self {
            Self::InputBytes => &mut budget.input_bytes,
            Self::Properties => &mut budget.properties,
            Self::Nodes => &mut budget.nodes,
            Self::ParserTokens => &mut budget.parser_tokens,
            Self::Sources => &mut budget.sources,
        }
    }
}
