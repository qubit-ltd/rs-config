// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for public behavior produced by configuration deserialization errors.

use std::error::Error;
use std::fmt;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::Property;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::DataType;
use qubit_value::Value;
use serde::Deserialize;
use serde::de;
use serde::de::Deserializer;
use serde::de::Unexpected;

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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredHost {
    host: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredNestedHost {
    nested: RequiredHost,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredPair {
    pair: (i32, i32),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RequiredControlField {
    #[serde(rename = "line\nbreak")]
    line_break: String,
}

#[derive(Debug)]
struct CustomSecretError;

impl<'de> Deserialize<'de> for CustomSecretError {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(de::Error::custom("secret-canary"))
    }
}

#[derive(Debug)]
struct InvalidTypeSecretError;

impl<'de> Deserialize<'de> for InvalidTypeSecretError {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SecretExpected;

        impl de::Expected for SecretExpected {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("expected secret-canary")
            }
        }

        Err(de::Error::invalid_type(
            Unexpected::Str("secret-canary"),
            &SecretExpected,
        ))
    }
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
        if message == "configuration value must have shape: a scalar")
    );
    assert!(error.source().is_none());
}

#[test]
fn test_missing_field_summary_preserves_schema_field_and_object_path() {
    let mut config = Config::new();
    config.set("app.port", 8080_u16).unwrap();

    let error = config.deserialize::<RequiredHost>("app").expect_err("host is required");

    assert_eq!(error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(error.path(), Some("app"));
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "missing required field: \"host\""
    ));
    assert!(error.source().is_none());
}

#[test]
fn test_nested_missing_field_summary_keeps_nested_object_path() {
    let mut config = Config::new();
    config.set("app.nested.port", 8080_u16).unwrap();

    let error = config
        .deserialize::<RequiredNestedHost>("app")
        .expect_err("nested host is required");

    assert_eq!(error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(error.path(), Some("app.nested"));
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "missing required field: \"host\""
    ));
}

#[test]
fn test_type_and_tuple_length_summaries_preserve_safe_categories() {
    let mut type_config = Config::new();
    type_config.set("app.value", vec![1_i32]).unwrap();
    let type_error = type_config
        .deserialize::<RequiredString>("app")
        .expect_err("a list cannot deserialize as a string");
    assert_eq!(type_error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(type_error.path(), Some("app.value"));
    assert!(matches!(
        &type_error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "configuration value must have shape: a scalar"
    ));

    let mut length_config = Config::new();
    length_config.set_default_read_policy(ReadPolicy::env_friendly());
    length_config.set("app.pair", "1").unwrap();
    let length_error = length_config
        .deserialize::<RequiredPair>("app")
        .expect_err("the tuple requires two elements");
    assert_eq!(length_error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(length_error.path(), Some("app.pair"));
    assert!(
        matches!(
            &length_error,
            ConfigError::DeserializeError { message, source: None, .. }
                if message == "configuration value has an invalid length: 1"
        ),
        "unexpected tuple length error: {length_error:?}"
    );
}

#[test]
fn test_nested_sequence_type_error_keeps_leaf_path_and_safe_category() {
    let mut config = Config::new();
    config
        .insert_property(
            "app.items",
            Property::new("app.items", Value::Json(serde_json::json!([{"value": [1]}]))).unwrap(),
        )
        .unwrap();

    let error = config
        .deserialize::<RequiredItems>("app")
        .expect_err("a nested list cannot deserialize as a string");

    assert_eq!(error.kind(), ConfigErrorKind::Deserialize);
    assert_eq!(error.path(), Some("app.items[0].value"));
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "configuration value must have shape: a scalar"
    ));
}

#[test]
fn test_missing_control_character_field_is_json_escaped() {
    let mut config = Config::new();
    config.set("app.other", "value").unwrap();

    let error = config
        .deserialize::<RequiredControlField>("app")
        .expect_err("renamed field is required");

    assert_eq!(error.path(), Some("app"));
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, .. }
            if message == "missing required field: \"line\\nbreak\""
    ));
}

#[test]
fn test_custom_and_invalid_type_errors_do_not_expose_secret_text() {
    const SECRET: &str = "secret-canary";
    for error in [
        {
            let mut config = Config::new();
            config.set("app.value", SECRET).unwrap();
            config
                .deserialize::<CustomSecretError>("app")
                .expect_err("custom deserializer rejects the value")
        },
        {
            let mut config = Config::new();
            config.set("app.value", SECRET).unwrap();
            config
                .deserialize::<InvalidTypeSecretError>("app")
                .expect_err("invalid type callback rejects the value")
        },
    ] {
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
        let mut source = error.source();
        while let Some(cause) = source {
            assert!(!cause.to_string().contains(SECRET));
            assert!(!format!("{cause:?}").contains(SECRET));
            source = cause.source();
        }
    }

    let mut config = Config::new();
    config.set("app.value", SECRET).unwrap();
    let error = config
        .deserialize::<CustomSecretError>("app")
        .expect_err("custom deserializer rejects the value");
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "configuration value does not match the requested type"
    ));

    let mut config = Config::new();
    config.set("app.value", SECRET).unwrap();
    let error = config
        .deserialize::<InvalidTypeSecretError>("app")
        .expect_err("invalid type callback rejects the value");
    assert!(matches!(
        &error,
        ConfigError::DeserializeError { message, source: None, .. }
            if message == "configuration value has an invalid type"
    ));
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
