// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;

struct ChildAccountingSource {
    amount: usize,
}

impl ConfigSource for ChildAccountingSource {
    fn source_id(&self) -> String {
        "child".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_input_bytes(5).build()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.consume_input_bytes(self.amount)
    }
}

struct RecoveringLocalFailureSource;

impl ConfigSource for RecoveringLocalFailureSource {
    fn source_id(&self) -> String {
        "recovering child".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_input_bytes(1).build()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let error = context
            .consume_input_bytes(2)
            .expect_err("the local budget should reject two bytes");
        assert_eq!(error.source_budget_id(), Some("recovering child"));
        context.consume_input_bytes(1)
    }
}

struct PartialLayerFailureSource;

impl ConfigSource for PartialLayerFailureSource {
    fn source_id(&self) -> String {
        "partial layer".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_input_bytes(1).build()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("partial.key", "must not escape")?;
        context.consume_input_bytes(2)
    }
}

#[test]
fn source_load_session_charges_local_and_ancestor_budgets_atomically() {
    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_input_bytes(3).build())
        .build();
    aggregate.add(ChildAccountingSource { amount: 2 });
    aggregate.add(ChildAccountingSource { amount: 2 });
    let error = aggregate
        .load()
        .expect_err("the aggregate budget should reject the second charge");

    assert_eq!(error.source_budget_id(), Some("composite configuration source"));
}

#[test]
fn source_load_session_reports_outermost_failing_budget_first() {
    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_input_bytes(1).build())
        .build();
    aggregate.add(ChildAccountingSource { amount: 2 });

    let error = aggregate
        .load()
        .expect_err("both aggregate and child budgets should reject the charge");

    assert_eq!(error.source_budget_id(), Some("composite configuration source"));
}

#[test]
fn source_load_session_failed_local_charge_leaves_every_scope_unchanged() {
    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_input_bytes(2).build())
        .build();
    aggregate.add(RecoveringLocalFailureSource);

    let _ = aggregate
        .load()
        .expect("a rejected local charge must not consume any scope");
}

#[test]
fn source_load_failure_discards_partial_layer_before_merge() {
    let mut config = Config::new();
    config.set("existing", "kept").unwrap();

    let error = config
        .merge_properties_from_source(&PartialLayerFailureSource)
        .expect_err("a failed source must not merge its partial layer");

    assert_eq!(error.source_budget_id(), Some("partial layer"));
    assert_eq!(config.get::<String>("existing").unwrap(), "kept");
    assert!(!config.contains("partial.key").unwrap());
}
