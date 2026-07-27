// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the stable versioned `Config` persistence wire format.

use qubit_config::Config;
use serde_json::json;

/// Verifies serialization emits the stable V1 envelope in deterministic order.
#[test]
fn test_config_wire_serialization_is_versioned_and_deterministic() {
    let mut first = Config::new();
    first
        .set("zebra", "last")
        .expect("setting the first property should succeed");
    first
        .set("apple", "first")
        .expect("setting the second property should succeed");

    let mut second = Config::new();
    second
        .set("apple", "first")
        .expect("setting the first property should succeed");
    second
        .set("zebra", "last")
        .expect("setting the second property should succeed");

    let first_json = serde_json::to_string(&first)
        .expect("serializing the first config should succeed");
    let second_json = serde_json::to_string(&second)
        .expect("serializing the second config should succeed");
    let wire: serde_json::Value = serde_json::from_str(&first_json)
        .expect("serialized config should be valid JSON");

    assert_eq!(wire["version"], json!(1));
    assert_eq!(first_json, second_json);
}

/// Verifies persisted payloads written before the V1 envelope remain readable.
#[test]
fn test_config_wire_deserializes_legacy_unversioned_payload() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut legacy = serde_json::to_value(&config)
        .expect("serializing the legacy-shaped config should succeed");
    legacy
        .as_object_mut()
        .expect("config wire should be a JSON object")
        .remove("version");

    let restored: Config = serde_json::from_value(legacy)
        .expect("legacy config wire should remain readable");

    assert_eq!(
        restored
            .get::<u16>("server.port")
            .expect("legacy property should retain its value"),
        8080,
    );
}

/// Verifies readers reject a wire revision they do not implement.
#[test]
fn test_config_wire_rejects_unknown_version() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = serde_json::to_value(&config)
        .expect("serializing config should succeed");
    wire["version"] = json!(2);

    let error = serde_json::from_value::<Config>(wire)
        .expect_err("unsupported config wire versions must be rejected");

    assert!(
        error
            .to_string()
            .contains("unsupported config wire version")
    );
}

/// Verifies V1 rejects fields that are outside its published wire contract.
#[test]
fn test_config_wire_rejects_unknown_v1_fields() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = serde_json::to_value(&config)
        .expect("serializing config should succeed");
    wire["future_field"] = json!(true);

    serde_json::from_value::<Config>(wire).expect_err(
        "unknown V1 fields must not silently deserialize as legacy",
    );
}

/// Verifies persisted map keys cannot disagree with their embedded property
/// name.
#[test]
fn test_config_wire_rejects_property_name_mismatch() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = serde_json::to_value(&config)
        .expect("serializing config should succeed");
    wire["properties"]["server.port"]["name"] = json!("wrong.name");

    let error = serde_json::from_value::<Config>(wire)
        .expect_err("property name mismatches must be rejected");

    assert!(error.to_string().contains("does not match property name"));
}

/// Verifies matching map/property names are still rejected when the common
/// name is not a canonical dotted key.
#[test]
fn test_config_wire_rejects_malformed_map_key() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = serde_json::to_value(&config)
        .expect("serializing config should succeed");
    let property = wire["properties"]
        .as_object_mut()
        .expect("properties should be an object")
        .remove("server.port")
        .expect("the serialized property should exist");
    let mut property = property;
    property["name"] = json!("bad..key");
    wire["properties"]
        .as_object_mut()
        .expect("properties should be an object")
        .insert("bad..key".to_string(), property);

    let error = serde_json::from_value::<Config>(wire)
        .expect_err("malformed config wire keys must be rejected");

    assert!(
        error.to_string().contains("bad..key"),
        "unexpected deserialization error: {error}"
    );
}
