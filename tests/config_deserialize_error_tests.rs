// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for public behavior produced by configuration deserialization errors.

use std::error::Error;

use qubit_config::{
    Config,
    ConfigError,
    ConfigErrorKind,
    Property,
    options::ReadOptions,
};
use qubit_datatype::{
    BlankStringPolicy,
    DataType,
};
use qubit_value::Value;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredString {
    value: String,
}

#[test]
fn test_deserialize_message_error_has_path_and_no_source() {
    let mut config = Config::new();
    config
        .set_null("app.value", DataType::String)
        .expect("setting null value should succeed");

    let error = config
        .deserialize::<RequiredString>("app")
        .expect_err("null string should fail during serde deserialization");

    assert_eq!(error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(error.path(), Some("app"));
    assert!(matches!(
        &error,
        ConfigError::DeserializeError {
            message,
            source: None,
            ..
        } if message == "configuration value does not match the requested type"
    ));
    assert!(error.source().is_none());
}

#[test]
fn test_deserialize_config_error_preserves_kind_and_leaf_path() {
    let mut config = Config::new();
    config.set_read_options(
        ReadOptions::default()
            .with_blank_string_policy(BlankStringPolicy::Reject),
    );
    config
        .insert_property(
            "app.value",
            Property::new("app.value", Value::Json(serde_json::json!(" "))),
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
