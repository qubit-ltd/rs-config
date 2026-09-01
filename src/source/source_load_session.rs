// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared local and aggregate accounting for configuration source loading.
// qubit-style: allow multiple-public-types

use qubit_budget::BudgetError;
use qubit_budget::InsufficientBudgetError;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceLimit;
use qubit_datatype::DataType;
use qubit_value::ValueContainer;

use super::SourceLimitKind;
use super::SourceLimits;
use crate::Config;
use crate::ConfigError;
use crate::ConfigName;
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
            properties: ResourceBudget::new(SourceLimitKind::PropertyCount, limits.max_properties()),
            nodes: ResourceBudget::new(SourceLimitKind::NodeCount, limits.max_nodes()),
            sources: ResourceBudget::new(SourceLimitKind::SourceCount, limits.max_sources()),
            depth: ResourceLimit::new(SourceLimitKind::NestingDepth, limits.max_nesting_depth()),
        }
    }
}

/// Selects one cumulative dimension from every active source budget.
#[derive(Clone, Copy)]
enum CumulativeDimension {
    InputBytes,
    Properties,
    Nodes,
    Sources,
}

impl CumulativeDimension {
    /// Borrows the selected cumulative resource budget.
    fn get(self, budget: &SourceLoadBudget) -> &ResourceBudget<SourceLimitKind, usize> {
        match self {
            Self::InputBytes => &budget.input_bytes,
            Self::Properties => &budget.properties,
            Self::Nodes => &budget.nodes,
            Self::Sources => &budget.sources,
        }
    }

    /// Mutably borrows the selected cumulative resource budget.
    fn get_mut(self, budget: &mut SourceLoadBudget) -> &mut ResourceBudget<SourceLimitKind, usize> {
        match self {
            Self::InputBytes => &mut budget.input_bytes,
            Self::Properties => &mut budget.properties,
            Self::Nodes => &mut budget.nodes,
            Self::Sources => &mut budget.sources,
        }
    }
}

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

/// Controlled output and accounting context supplied to a configuration source.
pub struct SourceLoadContext<'a> {
    session: SourceLoadSession<'a>,
    layer: Config,
}

impl SourceLoadContext<'static> {
    /// Creates a root context for one source load.
    pub(crate) fn new(source_id: String, limits: SourceLimits) -> Self {
        Self {
            session: SourceLoadSession::new(source_id, limits),
            layer: Config::new(),
        }
    }
}

impl<'a> SourceLoadContext<'a> {
    /// Charges input bytes to the current source and all aggregate ancestors.
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_input_bytes(amount)
    }

    /// Charges parsed nodes to the current source and all aggregate ancestors.
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_nodes(amount)
    }

    /// Charges admitted child sources to the current source and ancestors.
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_sources(amount)
    }

    /// Sets one output property after validating and charging the assignment.
    pub fn set<S>(&mut self, name: impl ConfigName, values: S) -> ConfigResult<()>
    where
        S: Into<ValueContainer>,
    {
        name.with_config_name(|name| {
            self.session.check_depth(name.split('.').count())?;
            self.session.consume_nodes(1)?;
            self.session.consume_properties(1)?;
            self.layer.set(name, values)
        })
    }

    /// Sets an explicitly typed null value after charging the assignment.
    pub fn set_null(&mut self, name: impl ConfigName, data_type: DataType) -> ConfigResult<()> {
        name.with_config_name(|name| {
            self.session.check_depth(name.split('.').count())?;
            self.session.consume_nodes(1)?;
            self.session.consume_properties(1)?;
            self.layer.set_null(name, data_type)
        })
    }

    /// Sets descriptive metadata on the source-owned layer.
    pub fn set_description(&mut self, description: Option<String>) {
        self.layer.set_description(description);
    }

    /// Sets the default read policy on the source-owned layer.
    pub fn set_default_read_policy(&mut self, policy: crate::options::ReadPolicy) {
        self.layer.set_default_read_policy(policy);
    }

    /// Returns the completed source layer to the crate-owned executor.
    pub(crate) fn finish(self) -> Config {
        self.layer
    }

    /// Creates a child context while retaining aggregate budget accounting.
    pub(crate) fn child<'child>(
        &'child mut self,
        source_id: String,
        limits: SourceLimits,
    ) -> SourceLoadContext<'child> {
        SourceLoadContext {
            session: self.session.child(source_id, limits),
            layer: Config::new(),
        }
    }

    /// Merges a completed child layer into this context transactionally.
    pub(crate) fn merge_layer(&mut self, layer: Config) -> ConfigResult<()> {
        self.layer.merge_properties(layer)
    }

    /// Borrows the legacy accounting session for built-in source parsers.
    pub(crate) fn session_mut(&mut self) -> &mut SourceLoadSession<'a> {
        &mut self.session
    }

    /// Replaces the layer produced by a built-in source parser.
    pub(crate) fn replace_layer(&mut self, layer: Config) {
        self.layer = layer;
    }
}
