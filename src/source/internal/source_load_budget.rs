// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;

use crate::source::SourceLimitKind;
use crate::source::SourceLimits;

/// Owned resource budgets for one source or composite scope.
#[must_use = "source load budgets must be retained for accounting"]
pub(in crate::source) struct SourceLoadBudget {
    /// Cumulative raw input bytes admitted in this source scope.
    pub(in crate::source) input_bytes: ResourceBudget<SourceLimitKind, usize>,
    /// Cumulative property assignments emitted in this source scope.
    pub(in crate::source) properties: ResourceBudget<SourceLimitKind, usize>,
    /// Cumulative structural nodes produced in this source scope.
    pub(in crate::source) nodes: ResourceBudget<SourceLimitKind, usize>,
    /// Cumulative parser tokens consumed in this source scope.
    pub(in crate::source) parser_tokens: ResourceBudget<SourceLimitKind, usize>,
    /// Cumulative child sources admitted in this source scope.
    pub(in crate::source) sources: ResourceBudget<SourceLimitKind, usize>,
    /// Per-input semantic nesting-depth limit for this source scope.
    pub(in crate::source) depth: ResourceLimit<SourceLimitKind, usize>,
    /// Per-input parser nesting-depth limit for this source scope.
    pub(in crate::source) parser_depth: ResourceLimit<SourceLimitKind, usize>,
}

impl SourceLoadBudget {
    /// Creates fresh resource counters from one source policy.
    ///
    /// # Parameters
    ///
    /// * `limits` - The limits used to initialize each counter.
    ///
    /// # Returns
    ///
    /// A budget with zero usage and the configured limits.
    pub(in crate::source) fn new(limits: SourceLimits) -> Self {
        Self {
            input_bytes: ResourceBudget::new(SourceLimitKind::InputBytes, limits.max_input_bytes()),
            properties: ResourceBudget::new(SourceLimitKind::PropertyCount, limits.max_properties()),
            nodes: ResourceBudget::new(SourceLimitKind::NodeCount, limits.max_nodes()),
            parser_tokens: ResourceBudget::new(SourceLimitKind::ParserTokenCount, limits.max_parser_tokens()),
            sources: ResourceBudget::new(SourceLimitKind::SourceCount, limits.max_sources()),
            depth: ResourceLimit::new(SourceLimitKind::NestingDepth, limits.max_nesting_depth()),
            parser_depth: ResourceLimit::new(SourceLimitKind::ParserNestingDepth, limits.max_parser_nesting_depth()),
        }
    }
}
