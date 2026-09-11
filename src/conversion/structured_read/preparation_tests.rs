// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Regression tests for borrowed structured input preparation.

use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_datatype::StructuredConversionLimits;
use qubit_value::ValueRef;

use super::node::ReadNode;
use super::prepared_read::prepare_read;
use crate::Config;
use crate::ConfigError;
use crate::ReadPolicy;

fn with_limits(config: &mut Config, limits: ConversionLimits) {
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
}

#[test]
fn nested_object_properties_merge_json_and_string_map_leaves() {
    use std::collections::HashMap;
    for use_map in [false, true] {
        let mut config = Config::new();
        config
            .set("tree.parent", serde_json::json!({"nested": {"left": "one"}}))
            .unwrap();
        if use_map {
            config
                .set(
                    "tree.parent.nested",
                    HashMap::from([("right".to_owned(), "two".to_owned())]),
                )
                .unwrap();
        } else {
            config
                .set("tree.parent.nested", serde_json::json!({"right": "two"}))
                .unwrap();
        }
        let value: serde_json::Value = super::deserialize_from_reader(&config, "tree", false, false).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"parent": {"nested": {"left": "one", "right": "two"}}})
        );
    }
    let mut config = Config::new();
    config
        .set("tree.map", HashMap::from([("left".to_owned(), "one".to_owned())]))
        .unwrap();
    config.set("tree.map.right", "two").unwrap();
    let value: serde_json::Value = super::deserialize_from_reader(&config, "tree", false, false).unwrap();
    assert_eq!(value, serde_json::json!({"map": {"left": "one", "right": "two"}}));
}

#[test]
fn source_text_and_rendered_number_budgets_fail_before_visiting() {
    let mut config = Config::new();
    config.set("text", "secret").unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_input_bytes(5).build())
            .build(),
    );
    let error = prepare_read(&config, "text", false).unwrap_err();
    assert_eq!(error.path(), Some("text"));
    assert!(!error.to_string().contains("secret"));
    config.set("number", 12345_i32).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_text_bytes(4).build())
            .build(),
    );
    let error = prepare_read(&config, "number", false).unwrap_err();
    assert_eq!(error.path(), Some("number"));
}

#[cfg(all(feature = "num-bigint", feature = "bigdecimal"))]
#[test]
fn rich_numeric_limits_reject_before_rendering() {
    use bigdecimal::BigDecimal;
    use num_bigint::BigInt;
    use qubit_datatype::NumericConversionLimits;
    let mut config = Config::new();
    config.set("integer", BigInt::from(1000)).unwrap();
    config.set("decimal", "1e100".parse::<BigDecimal>().unwrap()).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .numeric_limits(
                NumericConversionLimits::builder()
                    .max_big_integer_digits(2)
                    .max_big_decimal_scale_magnitude(2)
                    .build(),
            )
            .build(),
    );
    for name in ["integer", "decimal"] {
        let error = prepare_read(&config, name, false).unwrap_err();
        assert_eq!(error.path(), Some(name));
        assert!(matches!(error, ConfigError::ValueError { .. }));
    }
}

#[test]
fn full_input_admission_counts_unconsumed_fields_and_utf8_bytes() {
    let mut config = Config::new();
    config.set("tree.used", 1_i32).unwrap();
    config.set("tree.ignored", "界".repeat(100)).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(
                ConversionOperationLimits::builder()
                    .max_structured_payload_bytes(310)
                    .build(),
            )
            .build(),
    );
    assert!(prepare_read(&config, "tree", false).is_err());
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(
                ConversionOperationLimits::builder()
                    .max_structured_payload_bytes(312)
                    .build(),
            )
            .build(),
    );
    assert!(prepare_read(&config, "tree", false).is_ok());
}

#[test]
fn borrowed_json_and_string_maps_receive_complete_admission() {
    let mut config = Config::new();
    config.set("json", serde_json::json!({"nested": ["payload"]})).unwrap();
    config
        .set(
            "map",
            std::collections::HashMap::from([("key".to_owned(), "payload".to_owned())]),
        )
        .unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(
                ConversionOperationLimits::builder()
                    .max_structured_payload_bytes(9)
                    .build(),
            )
            .build(),
    );
    assert!(prepare_read(&config, "json", false).is_err());
    assert!(prepare_read(&config, "map", false).is_err());
}

#[test]
fn index_and_nested_payload_respect_depth_and_node_limits() {
    let mut config = Config::new();
    config.set("tree.a.b", 1_i32).unwrap();
    config.set("json", serde_json::json!({"a": {"b": 1}})).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_depth(2).build())
            .build(),
    );
    assert!(prepare_read(&config, "tree", false).is_err());
    assert!(prepare_read(&config, "json", false).is_err());
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_structured_nodes(2).build())
            .build(),
    );
    assert!(prepare_read(&config, "json", false).is_err());
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_structured_nodes(3).build())
            .build(),
    );
    assert!(prepare_read(&config, "json", false).is_ok());
}

#[test]
fn nonfinite_numbers_are_rejected_before_target_selection() {
    let mut config = Config::new();
    config.set("ignored", vec![1.0_f64, f64::NAN]).unwrap();
    let error = prepare_read(&config, "", false).unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert_eq!(error.path(), Some("ignored[1]"));
}

#[test]
fn map_keys_are_not_interpolated_and_values_are() {
    let mut config = Config::new();
    config.set("target", "expanded").unwrap();
    config
        .set(
            "map",
            std::collections::HashMap::from([("${absent}".to_owned(), "${target}".to_owned())]),
        )
        .unwrap();
    let prepared = prepare_read(&config, "map", true).unwrap();
    let ReadNode::StringMap(map) = &prepared.nodes[prepared.root] else {
        panic!()
    };
    assert!(map.contains_key("${absent}"));
    assert_eq!(
        prepared.overlays.values().map(String::as_str).collect::<Vec<_>>(),
        ["expanded"]
    );
}

#[cfg(feature = "url")]
#[test]
fn rich_values_never_become_interpolation_sources() {
    let url = url::Url::parse("custom:${absent}").unwrap();
    assert!(url.as_str().contains("${absent}"));
    let mut config = Config::new();
    config.set("url", url).unwrap();
    let prepared = prepare_read(&config, "url", true).unwrap();
    assert!(prepared.overlays.is_empty());
    assert!(matches!(
        prepared.nodes[prepared.root],
        ReadNode::Scalar(ValueRef::Url(_))
    ));
}

#[test]
fn zero_exact_and_exceeded_structure_bounds_are_enforced() {
    let mut config = Config::new();
    config.set("tree.a", true).unwrap();
    config.set("tree.b", false).unwrap();
    config.set("values", vec![1_i32, 2]).unwrap();
    for (limit, succeeds) in [(0, false), (1, false), (2, true)] {
        with_limits(
            &mut config,
            ConversionLimits::builder()
                .structured_limits(
                    StructuredConversionLimits::builder()
                        .max_map_entries(limit)
                        .max_sequence_items(limit)
                        .build(),
                )
                .build(),
        );
        assert_eq!(prepare_read(&config, "tree", false).is_ok(), succeeds);
        assert_eq!(prepare_read(&config, "values", false).is_ok(), succeeds);
    }
}

#[test]
fn long_keys_and_cumulative_keys_are_bounded_before_index_copying() {
    let mut config = Config::new();
    config.set("tree.abc", true).unwrap();
    config.set("tree.def", false).unwrap();
    for (limit, succeeds) in [(0, false), (2, false), (5, false), (6, true)] {
        with_limits(
            &mut config,
            ConversionLimits::builder()
                .operation_limits(
                    ConversionOperationLimits::builder()
                        .max_structured_payload_bytes(limit)
                        .build(),
                )
                .build(),
        );
        assert_eq!(prepare_read(&config, "tree", false).is_ok(), succeeds);
    }
}

#[test]
fn index_key_budget_rejects_input_bytes_before_admission() {
    let mut config = Config::new();
    config.set("tree.key", true).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_input_bytes(0).build())
            .build(),
    );
    assert!(prepare_read(&config, "tree", false).is_err());
}

#[test]
fn deep_json_is_rejected_by_iterative_admission() {
    let mut value = serde_json::json!(true);
    for _ in 0..512 {
        value = serde_json::Value::Array(vec![value]);
    }
    let mut config = Config::new();
    config.set("deep", value).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_depth(8).build())
            .build(),
    );
    let error = prepare_read(&config, "deep", false).unwrap_err();
    assert!(error.to_string().contains("StructuredDepth"));
}

#[test]
fn collection_budget_failure_retains_source_index_without_payload() {
    let mut config = Config::new();
    config.set("items", vec!["ok", "secret payload"]).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_text_bytes(2).build())
            .build(),
    );
    let error = prepare_read(&config, "items", false).unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert_eq!(error.path(), Some("items[1]"));
    assert!(!error.to_string().contains("secret payload"));
}

#[test]
fn exact_string_keeps_the_original_payload() {
    let mut config = Config::new();
    config.set("text", "x".repeat(65536)).unwrap();
    let original = config.get_property("text").unwrap().unwrap();
    let ValueRef::String(original) = original.value().as_scalar().unwrap().view() else {
        panic!()
    };
    let prepared = prepare_read(&config, "text", false).unwrap();
    let ReadNode::Scalar(ValueRef::String(actual)) = prepared.nodes[prepared.root] else {
        panic!()
    };
    assert_eq!(actual.as_ptr(), original.as_ptr());
    assert!(prepared.overlays.is_empty());
}

#[test]
fn dotted_subtree_has_sorted_children_and_absolute_path() {
    let mut config = Config::new();
    config.set("service.database.port", 5432_i32).unwrap();
    config.set("service.database.host", "localhost").unwrap();
    let section = config.section("service").unwrap();
    let prepared = prepare_read(&section, "database", false).unwrap();
    assert_eq!(prepared.root_path, "service.database");
    let ReadNode::Object(children) = &prepared.nodes[prepared.root] else {
        panic!()
    };
    assert_eq!(
        children.keys().map(String::as_str).collect::<Vec<_>>(),
        ["host", "port"]
    );
}

#[test]
fn exact_property_and_descendants_conflict_in_either_insertion_order() {
    for reverse in [false, true] {
        let mut config = Config::new();
        if reverse {
            config.set("server.port", 8080_i32).unwrap();
        }
        config.set("server", serde_json::json!({"host": "localhost"})).unwrap();
        if !reverse {
            config.set("server.port", 8080_i32).unwrap();
        }
        assert!(matches!(
            prepare_read(&config, "server", false),
            Err(ConfigError::KeyConflict { .. })
        ));
    }
}

#[test]
fn scalar_parent_rejects_nested_properties() {
    let mut config = Config::new();
    config.set("server", 8080_i32).unwrap();
    config.set("server.port", 8081_i32).unwrap();
    assert!(matches!(
        prepare_read(&config, "", false),
        Err(ConfigError::KeyConflict { .. })
    ));
}

#[test]
fn overlapping_objects_merge_distinct_leaves_and_reject_duplicates() {
    let mut config = Config::new();
    config
        .set("server.options", serde_json::json!({"nested": {"left": 1}}))
        .unwrap();
    config.set("server.options.nested.right", 2_i32).unwrap();
    let prepared = prepare_read(&config, "server", false).unwrap();
    let ReadNode::Object(root) = &prepared.nodes[prepared.root] else {
        panic!()
    };
    let ReadNode::Object(options) = &prepared.nodes[root["options"]] else {
        panic!()
    };
    let ReadNode::Object(nested) = &prepared.nodes[options["nested"]] else {
        panic!()
    };
    assert_eq!(nested.len(), 2);
    config.set("server.options.nested.left", 3_i32).unwrap();
    assert!(matches!(
        prepare_read(&config, "server", false),
        Err(ConfigError::KeyConflict { .. })
    ));
}

#[test]
fn interpolation_uses_selected_scope_and_keeps_unchanged_text_borrowed() {
    let mut config = Config::new();
    config.set("host", "root").unwrap();
    config.set("server.host", "local").unwrap();
    config.set("server.endpoint", "${host}").unwrap();
    config.set("server.literal", "x".repeat(65536)).unwrap();
    let prepared = prepare_read(&config, "server", true).unwrap();
    assert_eq!(
        prepared.overlays.values().map(String::as_str).collect::<Vec<_>>(),
        ["local"]
    );
    let ReadNode::Object(children) = &prepared.nodes[prepared.root] else {
        panic!()
    };
    let ReadNode::Scalar(ValueRef::String(literal)) = prepared.nodes[children["literal"]] else {
        panic!()
    };
    let original = config.get_property("server.literal").unwrap().unwrap();
    let ValueRef::String(original) = original.value().as_scalar().unwrap().view() else {
        panic!()
    };
    assert_eq!(literal.as_ptr(), original.as_ptr());
}

#[test]
fn nested_overlay_locations_distinguish_literal_dots_and_array_indices() {
    use super::node::SourceSegment;
    let mut config = Config::new();
    config.set("left", "one").unwrap();
    config.set("right", "two").unwrap();
    config
        .set(
            "json",
            serde_json::json!({"a.b": "${left}", "a": {"b": "${right}"}, "items": ["${left},${right}"]}),
        )
        .unwrap();
    let prepared = prepare_read(&config, "json", true).unwrap();
    assert_eq!(prepared.overlays.len(), 3);
    assert!(
        prepared
            .overlays
            .iter()
            .any(|(location, value)| location.segments == [SourceSegment::Key("a.b".into())] && value == "one")
    );
    assert!(prepared.overlays.iter().any(|(location, value)| location.segments
        == [SourceSegment::Key("a".into()), SourceSegment::Key("b".into())]
        && value == "two"));
    assert!(prepared.overlays.iter().any(|(location, value)| location.segments
        == [SourceSegment::Key("items".into()), SourceSegment::Index(0)]
        && value == "one,two"));
}

#[test]
fn interpolation_has_independent_before_and_after_admission() {
    let mut config = Config::new();
    config.set("v", "0123456789").unwrap();
    config.set("text", "${v}").unwrap();
    for (limit, succeeds) in [(4, false), (10, true)] {
        with_limits(
            &mut config,
            ConversionLimits::builder()
                .operation_limits(
                    ConversionOperationLimits::builder()
                        .max_structured_payload_bytes(limit)
                        .build(),
                )
                .build(),
        );
        assert_eq!(prepare_read(&config, "text", true).is_ok(), succeeds);
    }
}

#[test]
fn scalar_policy_missing_is_omitted_only_at_property_boundary() {
    use qubit_datatype::BlankStringPolicy;
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::TreatAsMissing)
            .build(),
    );
    config.set("tree.missing", " ").unwrap();
    config.set("tree.nested.missing", " ").unwrap();
    config.set("tree.items", vec![" "]).unwrap();
    let exact = prepare_read(&config, "tree.missing", false).unwrap();
    assert!(matches!(exact.nodes[exact.root], ReadNode::Missing(_)));
    let tree = prepare_read(&config, "tree", false).unwrap();
    let ReadNode::Object(children) = &tree.nodes[tree.root] else {
        panic!()
    };
    assert_eq!(children.keys().map(String::as_str).collect::<Vec<_>>(), ["items"]);
}
