// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for the stable versioned `Config` persistence wire format.

use qubit_budget::BudgetError;
use qubit_budget::JsonResource;
use qubit_budget::Observation;
use qubit_budget::ResourceLimit;
use qubit_config::Config;
use qubit_config::ConfigWireDecodeError;
use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_config::ConfigWireLimits;
use qubit_config::options::ReadPolicy;
use qubit_value::ValueWireEncodeError;
use serde_json::Value;
use serde_json::from_str;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_string;
use serde_json::to_value;
use serde_json::to_vec;

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

    let first_json =
        to_string(&first).expect("serializing the first config should succeed");
    let second_json = to_string(&second)
        .expect("serializing the second config should succeed");
    let wire: Value =
        from_str(&first_json).expect("serialized config should be valid JSON");

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

    let mut legacy = to_value(&config)
        .expect("serializing the legacy-shaped config should succeed");
    legacy
        .as_object_mut()
        .expect("config wire should be a JSON object")
        .remove("version");

    let restored: Config =
        from_value(legacy).expect("legacy config wire should remain readable");

    assert_eq!(
        restored
            .get::<u16>("server.port")
            .expect("legacy property should retain its value"),
        8080,
    );
}

/// Verifies legacy runtime policy data is accepted but never applied.
#[test]
fn test_config_wire_ignores_legacy_read_options() {
    let mut config = Config::new();
    config
        .set("server.port", "8080")
        .expect("setting the property should succeed");

    let mut legacy =
        to_value(&config).expect("serializing config should succeed");
    let object = legacy
        .as_object_mut()
        .expect("config wire should be a JSON object");
    object.remove("version");
    object.insert(
        "read_options".to_owned(),
        json!({
            "conversion": {"collection": {"split_scalar_strings": true}},
            "environment_fallback_enabled": true,
        }),
    );

    let restored: Config =
        from_value(legacy).expect("legacy wire with runtime data should load");

    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
}

/// Verifies readers reject a wire revision they do not implement.
#[test]
fn test_config_wire_rejects_unknown_version() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
    wire["version"] = json!(2);

    let error = from_value::<Config>(wire)
        .expect_err("unsupported config wire versions must be rejected");

    assert!(
        error
            .to_string()
            .contains("unsupported config wire version")
    );
}

#[test]
fn test_config_wire_limits_apply_to_nested_values() {
    let mut config = Config::new();
    config
        .set("values", vec![1_i32, 2])
        .expect("setting the collection should succeed");
    let input = to_vec(&config).expect("config should serialize");
    let limits = ConfigWireLimits::default();
    let decode = limits.json_decode();
    let value = decode.value_limits();
    let structure =
        value
            .structure_limits()
            .with_sequence_items_limit(ResourceLimit::new(
                JsonResource::SequenceItems,
                1,
            ));
    let decode = decode
        .with_input_bytes_limit(ResourceLimit::new(
            JsonResource::InputBytes,
            u64::try_from(input.len()).expect("input length must fit"),
        ))
        .with_value_limits(value.with_structure_limits(structure));
    let limits = limits.with_json_decode(decode);

    assert!(matches!(
        Config::decode_json_slice_with_limits(&input, limits),
        Err(ConfigWireDecodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::SequenceItems,
            observed: Observation::Exact(2),
            maximum: 1,
        }))
    ));
}

/// Verifies bounded encoding round-trips with the default limits.
#[test]
fn test_config_wire_bounded_encode_round_trips_with_default_limits() {
    let mut config = Config::with_description("bounded wire round trip");
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let encoded = config
        .encode_json_vec()
        .expect("default limits should encode the configuration");
    let restored = Config::decode_json_slice(&encoded)
        .expect("the bounded encoding should decode");

    assert_eq!(restored, config);
}

/// Verifies bounded encoding rejects an excessive property count.
#[test]
fn test_config_wire_bounded_encode_rejects_property_count() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");
    let limits = ConfigWireLimits::default().with_max_properties(0);

    assert!(matches!(
        config.encode_json_vec_with_limits(limits),
        Err(ConfigWireEncodeError::LimitExceeded {
            kind: ConfigWireLimitKind::Properties,
            value: 1,
            maximum: 0,
        })
    ));
}

/// Verifies bounded encoding rejects an excessive property-key length.
#[test]
fn test_config_wire_bounded_encode_rejects_property_key_bytes() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");
    let limits = ConfigWireLimits::default().with_max_property_key_bytes(5);

    assert!(matches!(
        config.encode_json_vec_with_limits(limits),
        Err(ConfigWireEncodeError::LimitExceeded {
            kind: ConfigWireLimitKind::PropertyKeyBytes,
            value: 11,
            maximum: 5,
        })
    ));
}

/// Verifies bounded encoding rejects an excessive final output size.
#[test]
fn test_config_wire_bounded_encode_rejects_final_output_bytes() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");
    let encoded = config
        .encode_json_vec()
        .expect("default limits should encode the configuration");
    let maximum =
        u64::try_from(encoded.len() - 1).expect("output length must fit");
    let limits = ConfigWireLimits::default();
    let encode =
        limits
            .json_encode()
            .with_output_bytes_limit(ResourceLimit::new(
                JsonResource::OutputBytes,
                maximum,
            ));
    let limits = limits.with_json_encode(encode);

    let result = config.encode_json_vec_with_limits(limits);
    assert!(
        matches!(
            &result,
            Err(ConfigWireEncodeError::Budget(BudgetError::Insufficient {
                resource: JsonResource::OutputBytes,
                limit,
                remaining: 0,
                requested: 1,
            })) if *limit == maximum
        ),
        "unexpected encoding result: {result:?}"
    );
}

/// Verifies bounded encoding rejects unsupported value representations.
#[test]
fn test_config_wire_bounded_encode_preflights_value_representation() {
    let mut config = Config::new();
    config
        .set("ratio", f64::NAN)
        .expect("setting the non-finite value should succeed");

    assert!(matches!(
        config.encode_json_vec(),
        Err(ConfigWireEncodeError::Value(
            ValueWireEncodeError::NonFiniteFloat { .. }
        ))
    ));
}

/// Verifies bounded decoding rejects malformed JSON during preflight.
#[test]
fn test_config_wire_bounded_decode_preflights_json_syntax() {
    let input = br#"{"version":1,"properties":{}"#;

    let result = Config::decode_json_slice(input);
    assert!(
        matches!(
            &result,
            Err(ConfigWireDecodeError::Json(error))
                if error.to_string() == "invalid JSON input"
        ),
        "unexpected decoding result: {result:?}"
    );
}

/// Verifies configuration invariants are reported after runtime decoding.
#[test]
fn test_config_wire_reports_configuration_invariant_after_decoding() {
    let mut config = Config::new();
    for index in 0..20 {
        config
            .set(format!("server.item{index}"), index as u64)
            .expect("setting the property should succeed");
    }

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
    wire["properties"]["server.item0"]["name"] = json!("bad.name");
    let input =
        to_vec(&wire).expect("serializing the wire object should succeed");
    let limits = ConfigWireLimits::default();
    let decode =
        limits
            .json_decode()
            .with_input_bytes_limit(ResourceLimit::new(
                JsonResource::InputBytes,
                u64::try_from(input.len()).expect("input length must fit"),
            ));
    let limits = limits.with_json_decode(decode);

    let result = Config::decode_json_slice_with_limits(&input, limits);
    assert!(matches!(
        result,
        Err(ConfigWireDecodeError::InvalidConfig(error))
            if error.contains("does not match property name")
    ));
}

/// Verifies V1 rejects fields that are outside its published wire contract.
#[test]
fn test_config_wire_rejects_unknown_v1_fields() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
    wire["future_field"] = json!(true);

    from_value::<Config>(wire).expect_err(
        "unknown V1 fields must not silently deserialize as legacy",
    );
}

/// Verifies legacy runtime policy data is not part of the versioned contract.
#[test]
fn test_config_wire_rejects_read_options_in_v1_payload() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
    wire["read_options"] = json!({"environment_fallback_enabled": true});

    from_value::<Config>(wire)
        .expect_err("versioned config wire must not contain runtime policies");
}

/// Verifies persisted map keys cannot disagree with their embedded property
/// name.
#[test]
fn test_config_wire_rejects_property_name_mismatch() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
    wire["properties"]["server.port"]["name"] = json!("wrong.name");

    let error = from_value::<Config>(wire)
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

    let mut wire =
        to_value(&config).expect("serializing config should succeed");
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

    let error = from_value::<Config>(wire)
        .expect_err("malformed config wire keys must be rejected");

    assert!(
        error.to_string().contains("bad..key"),
        "unexpected deserialization error: {error}"
    );
}
