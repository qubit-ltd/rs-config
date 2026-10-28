// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;
use qubit_datatype::DataType;

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

struct InvalidKeySource;

struct InvalidNullKeySource;

struct LimitedInvalidKeySource;

impl ConfigSource for InvalidKeySource {
    fn source_id(&self) -> String {
        "custom:invalid".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("bad..key", "unused")
    }
}

impl ConfigSource for InvalidNullKeySource {
    fn source_id(&self) -> String {
        "custom:null-invalid".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set_null("bad..key", DataType::String)
    }
}

impl ConfigSource for LimitedInvalidKeySource {
    fn source_id(&self) -> String {
        "custom:limited-invalid".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_properties(0).build()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("bad..key", "unused")
    }
}

struct NodeAndNullSource;

impl ConfigSource for NodeAndNullSource {
    fn source_id(&self) -> String {
        "node and null".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let consume_nodes = std::hint::black_box(SourceLoadContext::consume_nodes);
        consume_nodes(context, 1)?;
        context.set_null("optional.value", DataType::String)
    }
}

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
fn test_custom_assignment_retains_source_id() {
    let error = InvalidKeySource
        .load()
        .expect_err("invalid assignment key should reject the source");

    assert_eq!(error.kind(), ConfigErrorKind::InvalidKey);
    assert_eq!(error.path(), Some("bad..key"));
    assert_eq!(error.source_id(), Some("custom:invalid"));
}

#[test]
fn test_custom_null_assignment_retains_source_id() {
    let error = InvalidNullKeySource
        .load()
        .expect_err("invalid null assignment key should reject the source");

    assert_eq!(error.kind(), ConfigErrorKind::InvalidKey);
    assert_eq!(error.path(), Some("bad..key"));
    assert_eq!(error.source_id(), Some("custom:null-invalid"));
}

#[test]
fn test_nested_composite_invalid_key_reports_leaf_source() {
    let mut inner = CompositeConfigSource::new();
    inner.add(InvalidKeySource);
    let mut outer = CompositeConfigSource::new();
    outer.add(inner);

    let error = outer
        .load()
        .expect_err("invalid key from a composite leaf should reject loading");

    assert_eq!(error.kind(), ConfigErrorKind::InvalidKey);
    assert_eq!(error.path(), Some("bad..key"));
    assert_eq!(error.source_id(), Some("custom:invalid"));
}

#[test]
fn test_direct_config_set_invalid_key_has_no_source_id() {
    let mut config = Config::new();
    let error = config
        .set("bad..key", "unused")
        .expect_err("direct config assignment should reject an invalid key");

    assert_eq!(error.kind(), ConfigErrorKind::InvalidKey);
    assert_eq!(error.path(), Some("bad..key"));
    assert_eq!(error.source_id(), None);
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
    assert_eq!(error.source_id(), Some("partial layer"));
    assert_eq!(config.get::<String>("existing").unwrap(), "kept");
    assert!(!config.contains("partial.key").unwrap());
}

#[test]
fn test_assignment_budget_failure_keeps_source_limit_error() {
    let source = LimitedInvalidKeySource;
    let error = source
        .load()
        .expect_err("assignment budget should fail before key validation");

    assert_eq!(error.kind(), ConfigErrorKind::SourceLimitExceeded);
    assert_eq!(error.source_id(), Some("custom:limited-invalid"));
}

#[test]
fn source_load_context_exposes_node_charges_and_typed_nulls() {
    let config = NodeAndNullSource.load().unwrap();

    assert!(config.is_unset("optional.value").unwrap());
    assert_eq!(
        config.get_property("optional.value").unwrap().unwrap().data_type(),
        DataType::String
    );
}
