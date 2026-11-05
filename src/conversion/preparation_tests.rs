// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Regression tests for private structured-read preparation contracts.

use qubit_datatype::BlankStringPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_value::ValueRef;

use super::node::ReadNode;
use super::prepared_read::prepare_read;
use crate::Config;
use crate::ReadPolicy;

#[test]
fn test_scalar_policy_missing_is_omitted_only_at_property_boundary() {
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

#[test]
fn test_diagnostic_paths_encode_each_original_segment_unambiguously() {
    assert_eq!(super::path::config_path("alpha.beta.gamma"), "alpha.beta.gamma");
    assert_eq!(super::path::child_key("", "root.key"), "[\"root.key\"]");
    assert_eq!(super::path::child_key("root", "a[b]"), "root[\"a[b]\"]");
    assert_eq!(super::path::child_key("root", "a\"b\\c"), "root[\"a\\\"b\\\\c\"]");
    assert_eq!(super::path::child_key("root", "line\nfeed"), "root[\"line\\nfeed\"]");
    assert_eq!(super::path::child_key("root", "literal.dot"), "root[\"literal.dot\"]");
    assert_eq!(
        super::path::child_key("root", "nested").as_str(),
        super::path::config_path("root.nested")
    );
}

#[test]
fn test_input_admission_reports_string_map_and_missing_source_types() {
    use std::borrow::Cow;
    use std::collections::HashMap;

    use qubit_datatype::DataType;
    use qubit_value::ValueMissing;

    let options = ReadPolicy::builder()
        .conversion_limits(
            ConversionLimits::builder()
                .operation_limits(ConversionOperationLimits::builder().max_structured_nodes(0).build())
                .build(),
        )
        .build();
    let map = HashMap::new();
    let missing = ValueMissing::unset_scalar(DataType::Int32, DataType::Json);
    let json = serde_json::Value::Bool(true);
    for value in [
        super::read_view::ReadView::StringMap(&map),
        super::read_view::ReadView::Missing(&missing),
        super::read_view::ReadView::Scalar(ValueRef::Unset(DataType::String)),
        super::read_view::ReadView::Scalar(ValueRef::Json(&json)),
    ] {
        let visit = super::visit::Visit {
            value,
            location: None,
            path: Cow::Borrowed("value"),
            depth: 0,
            key: None,
            original_index: None,
        };
        let mut budget = super::prepared_input_budget::PreparedInputBudget::new(&options);
        assert!(budget.admit_visit(&visit, None).is_err());
    }
}

#[test]
fn test_exact_string_keeps_the_original_payload() {
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
fn test_nested_overlay_locations_distinguish_literal_dots_and_array_indices() {
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
fn test_interpolation_unchanged_text_keeps_source_pointer() {
    let mut config = Config::new();
    config.set("server.literal", "x".repeat(65536)).unwrap();
    let original = config.get_property("server.literal").unwrap().unwrap();
    let ValueRef::String(original) = original.value().as_scalar().unwrap().view() else {
        panic!()
    };
    let prepared = prepare_read(&config, "server", true).unwrap();
    let ReadNode::Object(children) = &prepared.nodes[prepared.root] else {
        panic!()
    };
    let ReadNode::Scalar(ValueRef::String(actual)) = prepared.nodes[children["literal"]] else {
        panic!()
    };
    assert_eq!(actual.as_ptr(), original.as_ptr());
    assert!(prepared.overlays.is_empty());
}
