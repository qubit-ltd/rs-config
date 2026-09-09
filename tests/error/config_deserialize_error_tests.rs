// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for public behavior produced by configuration deserialization errors.

use std::error::Error;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::Property;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::DataType;
use qubit_value::Value;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredString {
    value: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredItems {
    items: Vec<RequiredString>,
}

#[test]
fn test_unset_deserialize_error_retains_missing_source_and_path() {
    let mut config = Config::new();
    config
        .set_null("app.value", DataType::String)
        .expect("setting null value should succeed");

    let error = config
        .deserialize::<RequiredString>("app")
        .expect_err("null string should fail during serde deserialization");

    assert_eq!(error.kind(), ConfigErrorKind::Value);
    assert_eq!(error.path(), Some("app.value"));
    assert!(error.value_missing().unwrap().is_unset());
    assert!(error.source().is_some());
}

#[test]
fn test_nested_sequence_missing_error_has_leaf_path_and_index() {
    let mut config = Config::new();
    config
        .insert_property(
            "app.items",
            Property::new("app.items", Value::Json(serde_json::json!([{ "value": null }]))).unwrap(),
        )
        .expect("inserting nested items should succeed");

    let error = config
        .deserialize::<RequiredItems>("app")
        .expect_err("null nested string should fail deserialization");

    assert_eq!(error.kind(), ConfigErrorKind::Value);
    assert_eq!(error.path(), Some("app.items[0].value"));
    assert_eq!(error.source_index(), Some(0));
    assert!(error.value_missing().is_some());
}

#[test]
fn test_serde_shape_error_has_sanitized_message_without_source() {
    let mut config = Config::new();
    config.set("app.value", vec![1_i32]).unwrap();
    let error = config.deserialize::<RequiredString>("app").unwrap_err();
    assert_eq!(error.path(), Some("app.value"));
    assert!(
        matches!(&error, ConfigError::DeserializeError { message, source: None, .. }
        if message == "configuration value does not match the requested type")
    );
    assert!(error.source().is_none());
}

#[test]
fn test_deserialize_config_error_preserves_kind_and_leaf_path() {
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::Reject)
            .build(),
    );
    config
        .insert_property(
            "app.value",
            Property::new("app.value", Value::Json(serde_json::json!(" "))).unwrap(),
        )
        .expect("inserting property should succeed");

    let error = config
        .deserialize::<RequiredString>("app")
        .expect_err("blank string should be rejected by config conversion");

    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("app.value"));
    assert!(matches!(
        &error,
        ConfigError::ConversionError { key, .. } if key == "app.value"
    ));
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("Deserialization error at 'app'"));
}
