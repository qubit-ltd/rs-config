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
pub(in crate::source) struct SourceLoadBudget {
    pub(in crate::source) input_bytes: ResourceBudget<SourceLimitKind, usize>,
    pub(in crate::source) properties: ResourceBudget<SourceLimitKind, usize>,
    pub(in crate::source) nodes: ResourceBudget<SourceLimitKind, usize>,
    pub(in crate::source) sources: ResourceBudget<SourceLimitKind, usize>,
    pub(in crate::source) depth: ResourceLimit<SourceLimitKind, usize>,
}

impl SourceLoadBudget {
    /// Creates an unused budget from one source policy.
    pub(in crate::source) fn new(limits: SourceLimits) -> Self {
        Self {
            input_bytes: ResourceBudget::new(SourceLimitKind::InputBytes, limits.max_input_bytes()),
            properties: ResourceBudget::new(SourceLimitKind::PropertyCount, limits.max_properties()),
            nodes: ResourceBudget::new(SourceLimitKind::NodeCount, limits.max_nodes()),
            sources: ResourceBudget::new(SourceLimitKind::SourceCount, limits.max_sources()),
            depth: ResourceLimit::new(SourceLimitKind::NestingDepth, limits.max_nesting_depth()),
        }
    }
}
