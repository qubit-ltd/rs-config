// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;
use qubit_datatype::DataType;

struct ChildAccountingSource {
    amount: usize,
}

struct ParserAccountingSource {
    source_id: &'static str,
    tokens: usize,
    depth: usize,
    limits: SourceLimits,
}

struct RecoveringParserChargeSource {
    source_id: &'static str,
    local_limit: usize,
    rejected_amount: usize,
    accepted_amount: usize,
}

impl ConfigSource for RecoveringParserChargeSource {
    fn source_id(&self) -> String {
        self.source_id.to_owned()
    }
    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_parser_tokens(self.local_limit).build()
    }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let error = context
            .consume_parser_tokens(self.rejected_amount)
            .expect_err("the configured local parser limit should reject this charge");
        assert_eq!(error.source_budget_id(), Some(self.source_id));
        context.consume_parser_tokens(self.accepted_amount)
    }
}

impl ConfigSource for ParserAccountingSource {
    fn source_id(&self) -> String {
        self.source_id.to_owned()
    }
    fn limits(&self) -> SourceLimits {
        self.limits
    }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.consume_parser_tokens(self.tokens)?;
        context.check_parser_depth(self.depth)
    }
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
fn test_source_load_session_charges_local_and_ancestor_budgets_atomically() {
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
fn test_source_load_session_reports_outermost_failing_budget_first() {
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
fn test_source_load_session_failed_local_charge_leaves_every_scope_unchanged() {
    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_input_bytes(2).build())
        .build();
    aggregate.add(RecoveringLocalFailureSource);

    let _ = aggregate
        .load()
        .expect("a rejected local charge must not consume any scope");
}

#[test]
fn test_source_load_failure_discards_partial_layer_before_merge() {
    let mut config = Config::new();
    config
        .set("existing", "kept")
        .expect("initial config value should be valid");

    let error = config
        .merge_properties_from_source(&PartialLayerFailureSource)
        .expect_err("a failed source must not merge its partial layer");

    assert_eq!(error.source_budget_id(), Some("partial layer"));
    assert_eq!(error.source_id(), Some("partial layer"));
    assert_eq!(
        config
            .get::<String>("existing")
            .expect("existing value should remain readable"),
        "kept"
    );
    assert!(
        !config
            .contains("partial.key")
            .expect("partial key lookup should succeed")
    );
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
fn test_source_load_context_exposes_node_charges_and_typed_nulls() {
    let config = NodeAndNullSource
        .load()
        .expect("node accounting and typed null assignment should succeed");

    assert!(
        config
            .is_unset("optional.value")
            .expect("optional property lookup should succeed")
    );
    assert_eq!(
        config
            .get_property("optional.value")
            .expect("optional property lookup should succeed")
            .expect("typed null property should be present")
            .data_type(),
        DataType::String
    );
}

#[test]
fn test_parser_token_charge_is_atomic_across_ancestors() {
    struct ChargeSequence;
    impl ConfigSource for ChargeSequence {
        fn source_id(&self) -> String {
            "charge sequence".to_owned()
        }
        fn limits(&self) -> SourceLimits {
            SourceLimits::builder().max_parser_tokens(5).build()
        }
        fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
            context.consume_parser_tokens(2)?;
            let error = context
                .consume_parser_tokens(2)
                .expect_err("the ancestor has one token remaining");
            assert_eq!(error.source_id(), Some("charge sequence"));
            assert_eq!(error.source_budget_id(), Some("composite configuration source"));
            context.consume_parser_tokens(1)
        }
    }
    let mut sequence = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_parser_tokens(3).build())
        .build();
    sequence.add(ChargeSequence);
    let _ = sequence
        .load()
        .expect("failed grouped charge must leave one token available");

    let mut composite = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_parser_tokens(3).build())
        .build();
    composite.add(ParserAccountingSource {
        source_id: "parser child",
        tokens: 2,
        depth: 0,
        limits: SourceLimits::builder().max_parser_tokens(5).build(),
    });
    composite.add(ParserAccountingSource {
        source_id: "parser child",
        tokens: 2,
        depth: 0,
        limits: SourceLimits::builder().max_parser_tokens(5).build(),
    });
    let error = composite
        .load()
        .expect_err("ancestor charge should reject sibling's second charge");
    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::ParserTokenCount,
            limit: 3,
            observed_at_least: 4,
            ..
        }
    ));
    assert_eq!(error.source_id(), Some("parser child"));
    assert_eq!(error.source_budget_id(), Some("composite configuration source"));

    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_parser_tokens(2).build())
        .build();
    aggregate.add(RecoveringParserChargeSource {
        source_id: "local parser",
        local_limit: 1,
        rejected_amount: 2,
        accepted_amount: 1,
    });
    aggregate.add(ParserAccountingSource {
        source_id: "later parser",
        tokens: 2,
        depth: 0,
        limits: SourceLimits::builder().max_parser_tokens(5).build(),
    });
    let error = aggregate
        .load()
        .expect_err("the successful charge uses one ancestor token");
    assert_eq!(error.source_id(), Some("later parser"));
    assert_eq!(error.source_budget_id(), Some("composite configuration source"));
}

#[test]
fn test_parser_depth_reports_rejecting_scope() {
    let mut composite = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_parser_nesting_depth(2).build())
        .build();
    composite.add(ParserAccountingSource {
        source_id: "deep parser",
        tokens: 0,
        depth: 3,
        limits: SourceLimits::builder().max_parser_nesting_depth(4).build(),
    });
    let error = composite
        .load()
        .expect_err("ancestor depth limit should reject the input");
    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::ParserNestingDepth,
            limit: 2,
            observed_at_least: 3,
            ..
        }
    ));
    assert_eq!(error.source_id(), Some("deep parser"));
    assert_eq!(error.source_budget_id(), Some("composite configuration source"));

    let mut same_depth = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_parser_nesting_depth(3).build())
        .build();
    same_depth.add(ParserAccountingSource {
        source_id: "depth one",
        tokens: 0,
        depth: 3,
        limits: SourceLimits::default(),
    });
    same_depth.add(ParserAccountingSource {
        source_id: "depth two",
        tokens: 0,
        depth: 3,
        limits: SourceLimits::default(),
    });
    let _ = same_depth
        .load()
        .expect("parser depth is checked per input, not accumulated");
}

#[test]
fn test_parser_budget_failure_preserves_merged_config() {
    let source = ParserAccountingSource {
        source_id: "limited parser",
        tokens: 1,
        depth: 0,
        limits: SourceLimits::builder().max_parser_tokens(0).build(),
    };
    let mut config = Config::new();
    config.set("existing", "kept").expect("initial value should be valid");

    let error = config
        .merge_properties_from_source(&source)
        .expect_err("parser token budget should reject the source");

    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::ParserTokenCount,
            limit: 0,
            observed_at_least: 1,
            ..
        }
    ));
    assert_eq!(
        config.get::<String>("existing").expect("existing value should remain"),
        "kept"
    );
    assert_eq!(config.len(), 1);
}
