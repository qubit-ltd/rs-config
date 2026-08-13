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
            input_bytes: ResourceBudget::new(
                SourceLimitKind::InputBytes,
                limits.max_input_bytes(),
            ),
            properties: ResourceBudget::new(
                SourceLimitKind::PropertyCount,
                limits.max_properties(),
            ),
            nodes: ResourceBudget::new(
                SourceLimitKind::NodeCount,
                limits.max_nodes(),
            ),
            sources: ResourceBudget::new(
                SourceLimitKind::SourceCount,
                limits.max_sources(),
            ),
            depth: ResourceLimit::new(
                SourceLimitKind::NestingDepth,
                limits.max_nesting_depth(),
            ),
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
        if self.ancestors.is_empty() {
            return match self.local.input_bytes.try_consume(amount) {
                Ok(()) => Ok(()),
                Err(source) => {
                    Err(self.limit_error(self.source_id.clone(), source))
                }
            };
        }
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
        if self.ancestors.is_empty() {
            return match self.local.properties.try_consume(amount) {
                Ok(()) => Ok(()),
                Err(source) => {
                    Err(self.limit_error(self.source_id.clone(), source))
                }
            };
        }
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
        if self.ancestors.is_empty() {
            return match self.local.nodes.try_consume(amount) {
                Ok(()) => Ok(()),
                Err(source) => {
                    Err(self.limit_error(self.source_id.clone(), source))
                }
            };
        }
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
        if self.ancestors.is_empty() {
            return match self.local.sources.try_consume(amount) {
                Ok(()) => Ok(()),
                Err(source) => {
                    Err(self.limit_error(self.source_id.clone(), source))
                }
            };
        }
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
            budget.depth.check(depth).map_err(|source| {
                self.limit_error(self.ancestor_ids[index].clone(), source)
            })?;
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
        let limit = source
            .maximum()
            .or(source.limit())
            .expect("source budget errors always carry a limit");
        let observed_at_least = source.observed_lower_bound().or_else(|| {
            Some(
                limit
                    .saturating_sub(
                        source
                            .remaining()
                            .expect("cumulative budget errors carry remaining"),
                    )
                    .saturating_add(
                        source
                            .requested()
                            .expect("cumulative budget errors carry request"),
                    ),
            )
        });
        ConfigError::SourceLimitExceeded {
            source_id: self.source_id.clone(),
            budget_id,
            kind: *source.resource(),
            limit,
            observed_at_least: observed_at_least
                .expect("source budget errors always carry an observation"),
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
    pub fn set<S>(
        &mut self,
        name: impl ConfigName,
        values: S,
    ) -> ConfigResult<()>
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
    pub fn set_null(
        &mut self,
        name: impl ConfigName,
        data_type: DataType,
    ) -> ConfigResult<()> {
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
    pub fn set_default_read_policy(
        &mut self,
        policy: crate::options::ReadPolicy,
    ) {
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
