// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for the stable versioned `Config` persistence wire format.

use qubit_budget::BudgetError;
use qubit_budget::Observation;
use qubit_budget::ResourceLimit;
use qubit_budget::StructureLimits;
use qubit_budget::json::JsonDecodeLimits;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueLimits;
use qubit_config::Config;
use qubit_config::ConfigWireDecodeError;
use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_config::ConfigWireLimits;
use qubit_config::options::ReadPolicy;
use qubit_json::decode::JsonSyntaxErrorReason;
use qubit_value::ValueWireEncodeError;
use serde::Deserialize;
use serde::Deserializer;
use serde::forward_to_deserialize_any;
use serde::de::IntoDeserializer;
use serde::de::Visitor;
use serde::de::value::Error as ValueDeserializerError;
use serde::de::value::MapDeserializer;
use serde::de::value::SeqDeserializer;
use serde_json::Value;
use serde_json::error::Category;
use serde_json::from_slice;
use serde_json::from_str;
use serde_json::from_value;
use serde_json::json;
use serde_json::to_string;
use serde_json::to_value;
use serde_json::to_vec;

/// Serde events used to verify format-independent `Config` deserialization.
enum WireEvent {
    Bool(bool),
    I64(i64),
    I128(i128),
    U64(u64),
    U128(u128),
    F64(f64),
    Str(String),
    String(String),
    None,
    Unit,
    Some(Box<Self>),
    Newtype(Box<Self>),
    Seq(Vec<Self>),
    Map(Vec<(Self, Self)>),
    Bytes(&'static [u8]),
}

impl<'de> IntoDeserializer<'de, ValueDeserializerError> for WireEvent {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for WireEvent {
    type Error = ValueDeserializerError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bool(value) => visitor.visit_bool(value),
            Self::I64(value) => visitor.visit_i64(value),
            Self::I128(value) => visitor.visit_i128(value),
            Self::U64(value) => visitor.visit_u64(value),
            Self::U128(value) => visitor.visit_u128(value),
            Self::F64(value) => visitor.visit_f64(value),
            Self::Str(value) => visitor.visit_str(&value),
            Self::String(value) => visitor.visit_string(value),
            Self::None => visitor.visit_none(),
            Self::Unit => visitor.visit_unit(),
            Self::Some(value) => visitor.visit_some(*value),
            Self::Newtype(value) => visitor.visit_newtype_struct(*value),
            Self::Seq(values) => visitor.visit_seq(SeqDeserializer::new(values.into_iter())),
            Self::Map(entries) => visitor.visit_map(MapDeserializer::new(entries.into_iter())),
            Self::Bytes(value) => visitor.visit_bytes(value),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}

#[derive(Default)]
struct WireEventCounts {
    negative_numbers: usize,
    positive_numbers: usize,
    strings: usize,
    nulls: usize,
}

fn into_wire_events(value: Value, counts: &mut WireEventCounts) -> WireEvent {
    match value {
        Value::Null => {
            counts.nulls += 1;
            if counts.nulls == 1 {
                WireEvent::None
            } else {
                WireEvent::Unit
            }
        }
        Value::Bool(value) => WireEvent::Bool(value),
        Value::Number(number) => {
            if let Some(value) = number.as_i64().filter(|value| *value < 0) {
                counts.negative_numbers += 1;
                if counts.negative_numbers == 1 {
                    WireEvent::I128(i128::from(value))
                } else {
                    WireEvent::I64(value)
                }
            } else if let Some(value) = number.as_u64() {
                counts.positive_numbers += 1;
                if counts.positive_numbers == 1 {
                    WireEvent::Newtype(Box::new(WireEvent::U128(u128::from(value))))
                } else {
                    WireEvent::U64(value)
                }
            } else {
                WireEvent::F64(number.as_f64().expect("the JSON number should fit in f64"))
            }
        }
        Value::String(value) => {
            counts.strings += 1;
            let event = if counts.strings.is_multiple_of(2) {
                WireEvent::String(value)
            } else {
                WireEvent::Str(value)
            };
            if counts.strings == 1 {
                WireEvent::Some(Box::new(event))
            } else {
                event
            }
        }
        Value::Array(values) => WireEvent::Seq(
            values
                .into_iter()
                .map(|value| into_wire_events(value, counts))
                .collect(),
        ),
        Value::Object(values) => WireEvent::Map(
            values
                .into_iter()
                .map(|(key, value)| (WireEvent::String(key), into_wire_events(value, counts)))
                .collect(),
        ),
    }
}

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

    let first_json = to_string(&first).expect("serializing the first config should succeed");
    let second_json = to_string(&second).expect("serializing the second config should succeed");
    let wire: Value = from_str(&first_json).expect("serialized config should be valid JSON");

    assert_eq!(wire["version"], json!(1));
    assert_eq!(first_json, second_json);
}

/// Verifies persisted payloads written before the V1 envelope remain readable.
#[test]
fn test_config_wire_deserializes_unversioned_payload() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut legacy = to_value(&config).expect("serializing the legacy-shaped config should succeed");
    legacy
        .as_object_mut()
        .expect("config wire should be a JSON object")
        .remove("version");

    let restored: Config = from_value(legacy).expect("legacy config wire should remain readable");

    assert_eq!(
        restored
            .get::<u16>("server.port")
            .expect("legacy property should retain its value"),
        8080,
    );
}

#[test]
fn test_ordinary_deserialize_enforces_default_property_key_budget() {
    let key = "k".repeat(usize::try_from(ConfigWireLimits::DEFAULT_MAX_PROPERTY_KEY_BYTES).unwrap() + 1);
    let mut config = Config::new();
    config
        .set(&key, "value")
        .expect("the domain model accepts a long canonical key");
    let input = to_vec(&config).expect("the config should serialize");

    let error = from_slice::<Config>(&input).expect_err("ordinary Deserialize should apply default decoded budgets");

    assert!(error.to_string().contains("PropertyKeyBytes"));
}

#[test]
fn test_ordinary_deserialize_does_not_claim_raw_input_accounting() {
    let config = Config::new();
    let mut input = vec![b' '; usize::try_from(ConfigWireLimits::DEFAULT_MAX_INPUT_BYTES).unwrap()];
    input.extend_from_slice(&to_vec(&config).expect("the config should serialize"));

    let _ = from_slice::<Config>(&input).expect("ordinary Deserialize should only enforce decoded-value budgets");
    let error = Config::decode_json_slice(&input).expect_err("the bounded API should reject excessive raw input");
    assert!(matches!(
        error,
        ConfigWireDecodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::InputBytes,
            ..
        }) | ConfigWireDecodeError::Budget(BudgetError::Insufficient {
            resource: JsonResource::InputBytes,
            ..
        })
    ));
}

#[test]
fn test_ordinary_deserialize_enforces_default_decoded_string_budget() {
    let mut config = Config::new();
    config
        .set(
            "value",
            "x".repeat(usize::try_from(ConfigWireLimits::DEFAULT_MAX_STRING_BYTES).unwrap() + 1),
        )
        .expect("the domain model accepts the long string");
    let input = to_vec(&config).expect("the config should serialize");

    let error = from_slice::<Config>(&input).expect_err("ordinary Deserialize should enforce decoded string limits");

    assert!(error.to_string().contains("StringBytes"));
}

#[test]
fn test_ordinary_deserialize_enforces_default_depth_budget() {
    let mut nested = json!(true);
    for _ in 0..ConfigWireLimits::DEFAULT_MAX_DEPTH {
        nested = json!({"nested": nested});
    }
    let mut config = Config::new();
    config
        .set("nested", nested)
        .expect("the nested JSON value should be accepted by the domain model");
    let input = to_vec(&config).expect("the config should serialize");

    let error = from_slice::<Config>(&input).expect_err("ordinary Deserialize should enforce depth limits");
    assert!(error.to_string().contains("Depth"), "unexpected error: {error}");
}

#[test]
fn test_ordinary_deserialize_enforces_default_node_budget_incrementally() {
    let mut config = Config::new();
    for index in 0..ConfigWireLimits::DEFAULT_MAX_PROPERTIES {
        config
            .set(format!("values{index}"), vec![0_i32; 25])
            .expect("the large collection should be accepted by the domain model");
    }
    let input = to_vec(&config).expect("the config should serialize");

    let error = from_slice::<Config>(&input).expect_err("ordinary Deserialize should enforce node limits");
    assert!(error.to_string().contains("Nodes"), "unexpected error: {error}");
}

#[test]
fn test_ordinary_deserialize_accounts_array_values_and_missing_properties() {
    let mut config = Config::new();
    config
        .set("values", vec![1_i32, 2_i32])
        .expect("the collection should be set");
    let wire = to_value(&config).expect("the config should serialize");

    let restored: Config = from_value(wire).expect("ordinary Deserialize should admit arrays");
    assert_eq!(restored.get::<Vec<i32>>("values").unwrap(), vec![1, 2]);

    let empty: Config =
        from_value(json!({"version": 1})).expect("a versioned wire value may omit its defaulted property map");
    assert!(empty.is_empty());
}

#[test]
fn test_ordinary_deserialize_accepts_equivalent_serde_value_events() {
    let mut expected = Config::builder().description("Serde event coverage").build();
    expected
        .set("numbers.first", -7_i32)
        .expect("the first signed value should be set");
    expected
        .set("numbers.second", -9_i64)
        .expect("the second signed value should be set");
    expected
        .set("numbers.ratio", 1.5_f64)
        .expect("the floating-point value should be set");
    expected
        .set("flags", vec![true, false])
        .expect("the Boolean sequence should be set");
    expected
        .set("labels", vec!["alpha", "beta"])
        .expect("the string sequence should be set");

    let wire = to_value(&expected).expect("the configuration should serialize");
    let mut counts = WireEventCounts::default();
    let events = into_wire_events(wire, &mut counts);

    assert!(
        counts.negative_numbers >= 2,
        "the fixture should exercise i128 and i64 events"
    );
    assert!(
        counts.positive_numbers >= 2,
        "the fixture should exercise u128 and u64 events"
    );
    assert!(
        counts.strings >= 2,
        "the fixture should exercise borrowed and owned string events"
    );
    assert!(counts.nulls >= 2, "the fixture should exercise none and unit events");

    let restored = <Config as Deserialize>::deserialize(events).expect("equivalent Serde events should decode");
    assert_eq!(restored, expected);
}

#[test]
fn test_ordinary_deserialize_rejects_unrepresentable_serde_value_events() {
    let cases = [
        (
            WireEvent::I128(i128::MAX),
            "JSON integer is outside the supported 64-bit range",
        ),
        (
            WireEvent::U128(u128::MAX),
            "JSON integer is outside the supported 64-bit range",
        ),
        (
            WireEvent::F64(f64::NAN),
            "non-finite float is not representable as JSON",
        ),
        (
            WireEvent::Bytes(b"not a wire value"),
            "a JSON value with unique object keys",
        ),
    ];

    for (events, expected_message) in cases {
        let error =
            <Config as Deserialize>::deserialize(events).expect_err("the unsupported Serde event must be rejected");
        assert!(
            error.to_string().contains(expected_message),
            "unexpected event error: {error}",
        );
    }
}

#[test]
fn test_bounded_decode_uses_custom_config_wire_domain_budget() {
    let mut config = Config::new();
    config.set("value", "x").expect("the property should be set");
    let input = to_vec(&config).expect("the config should serialize");
    let limits = ConfigWireLimits::builder().max_properties(0).build();

    assert!(matches!(
        Config::decode_json_slice_with_limits(&input, limits),
        Err(ConfigWireDecodeError::LimitExceeded {
            kind: ConfigWireLimitKind::Properties,
            value: 1,
            maximum: 0,
        })
    ));
}

#[test]
fn test_bounded_decode_uses_custom_property_key_budget() {
    let mut config = Config::new();
    config.set("long-key", "value").unwrap();
    let input = to_vec(&config).expect("the config should serialize");
    let limits = ConfigWireLimits::builder().max_property_key_bytes(3).build();

    assert!(matches!(
        Config::decode_json_slice_with_limits(&input, limits),
        Err(ConfigWireDecodeError::LimitExceeded {
            kind: ConfigWireLimitKind::PropertyKeyBytes,
            value: 8,
            maximum: 3,
        })
    ));
}

#[test]
fn test_ordinary_deserialize_enforces_default_property_count_budget() {
    let mut config = Config::new();
    for index in 0..=ConfigWireLimits::DEFAULT_MAX_PROPERTIES {
        config.set(format!("item{index}"), index).unwrap();
    }
    let input = to_vec(&config).expect("the large config should serialize");

    let error = from_slice::<Config>(&input)
        .expect_err("ordinary Deserialize should reject a map beyond the shared entry budget");

    assert!(
        error.to_string().contains("MapEntries"),
        "unexpected budget error: {error}"
    );
}

/// Verifies legacy runtime policy data is accepted but never applied.
#[test]
fn test_config_wire_ignores_unversioned_read_options() {
    let mut config = Config::new();
    config
        .set("server.port", "8080")
        .expect("setting the property should succeed");

    let mut legacy = to_value(&config).expect("serializing config should succeed");
    let object = legacy.as_object_mut().expect("config wire should be a JSON object");
    object.remove("version");
    object.insert(
        "read_options".to_owned(),
        json!({
            "conversion": {"collection": {"split_scalar_strings": true}},
            "environment_fallback_enabled": true,
        }),
    );

    let restored: Config = from_value(legacy).expect("legacy wire with runtime data should load");

    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
}

/// Verifies readers reject a wire revision they do not implement.
#[test]
fn test_config_wire_rejects_unknown_version() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = to_value(&config).expect("serializing config should succeed");
    wire["version"] = json!(2);

    let error = from_value::<Config>(wire).expect_err("unsupported config wire versions must be rejected");

    assert!(error.to_string().contains("unsupported config wire version"));
}

#[test]
fn test_config_wire_limits_apply_to_nested_values() {
    let mut config = Config::new();
    config
        .set("values", vec![1_i32, 2])
        .expect("setting the collection should succeed");
    let input = to_vec(&config).expect("config should serialize");
    let limits = ConfigWireLimits::default();
    let structure = StructureLimits::builder()
        .sequence_items_limit(ResourceLimit::new(JsonResource::SequenceItems, 1))
        .build();
    let value = JsonValueLimits::builder().structure_limits(structure).build();
    let decode = JsonDecodeLimits::builder()
        .input_bytes_limit(ResourceLimit::new(
            JsonResource::InputBytes,
            u64::try_from(input.len()).expect("input length must fit"),
        ))
        .value_limits(value)
        .build();
    let limits = ConfigWireLimits::builder_from(&limits).json_decode(decode).build();

    assert!(matches!(
        Config::decode_json_slice_with_limits(&input, limits),
        Err(ConfigWireDecodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::SequenceItems,
            observed: Observation::Exact(2),
            maximum: 1,
        }))
    ));
}

fn assert_decode_rejects_resource(input: &[u8], decode: JsonDecodeLimits<JsonResource, u64>, resource: JsonResource) {
    let limits = ConfigWireLimits::builder_from(&ConfigWireLimits::default())
        .json_decode(decode)
        .build();
    let error = Config::decode_json_slice_with_limits(input, limits)
        .expect_err("the configured JSON budget must reject this input");
    let ConfigWireDecodeError::Budget(error) = error else {
        panic!("expected a budget error, got {error:?}");
    };
    assert_eq!(error.resource(), &resource);
}

#[test]
fn test_config_wire_decode_enforces_each_json_budget_dimension() {
    let empty = to_vec(&Config::new()).expect("the empty config should serialize");
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_input_bytes(0).build(),
        JsonResource::InputBytes,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_depth(0).build(),
        JsonResource::Depth,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_nodes(0).build(),
        JsonResource::Nodes,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_map_entries(0).build(),
        JsonResource::MapEntries,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_key_bytes(0).build(),
        JsonResource::KeyBytes,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_number_bytes(0).build(),
        JsonResource::NumberBytes,
    );
    assert_decode_rejects_resource(
        &empty,
        JsonDecodeLimits::builder().max_payload_bytes(0).build(),
        JsonResource::PayloadBytes,
    );

    let mut config = Config::builder().description("description").build();
    config
        .set("values", vec![1_i32, 2])
        .expect("setting the collection should succeed");
    let input = to_vec(&config).expect("the config should serialize");
    assert_decode_rejects_resource(
        &input,
        JsonDecodeLimits::builder().max_sequence_items(1).build(),
        JsonResource::SequenceItems,
    );
    assert_decode_rejects_resource(
        &input,
        JsonDecodeLimits::builder().max_string_bytes(0).build(),
        JsonResource::StringBytes,
    );
}

#[test]
fn test_config_wire_decode_rejects_nested_values_over_depth_limit() {
    let mut config = Config::new();
    config
        .set("values", vec![1_i32, 2])
        .expect("setting the collection should succeed");
    let input = to_vec(&config).expect("the config should serialize");

    assert_decode_rejects_resource(
        &input,
        JsonDecodeLimits::builder().max_depth(3).build(),
        JsonResource::Depth,
    );
}

/// Verifies bounded encoding round-trips with the default limits.
#[test]
fn test_config_wire_bounded_encode_round_trips_with_default_limits() {
    let mut config = Config::builder().description("bounded wire round trip").build();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let encoded = config
        .encode_json_vec()
        .expect("default limits should encode the configuration");
    let restored = Config::decode_json_slice(&encoded).expect("the bounded encoding should decode");

    assert_eq!(restored, config);
}

/// Verifies bounded encoding rejects an excessive property count.
#[test]
fn test_config_wire_bounded_encode_rejects_property_count() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");
    let limits = ConfigWireLimits::builder().max_properties(0).build();

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
    let limits = ConfigWireLimits::builder().max_property_key_bytes(5).build();

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
    let maximum = u64::try_from(encoded.len() - 1).expect("output length must fit");
    let limits = ConfigWireLimits::default();
    let encode = JsonEncodeLimits::builder()
        .output_bytes_limit(ResourceLimit::new(JsonResource::OutputBytes, maximum))
        .build();
    let limits = ConfigWireLimits::builder_from(&limits).json_encode(encode).build();

    let result = config.encode_json_vec_with_limits(limits);
    assert!(
        matches!(
            &result,
            Err(ConfigWireEncodeError::Budget(BudgetError::Insufficient {
                resource: JsonResource::OutputBytes,
                limit,
                remaining,
                requested,
            })) if *limit == maximum
                && *remaining == maximum
                && *requested == maximum + 1
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
            Err(ConfigWireDecodeError::Syntax(error))
                if error.reason() == JsonSyntaxErrorReason::UnexpectedEnd
                    && error.offset() == 28
                    && error.line() == 1
                    && error.column() == 29
        ),
        "unexpected decoding result: {result:?}"
    );
}

/// Verifies typed JSON decoding exposes only safe Serde error metadata.
#[test]
fn test_config_wire_bounded_decode_preserves_json_error_metadata() {
    let error = Config::decode_json_slice(br#"{"version":true,"properties":{}}"#)
        .expect_err("an invalid wire version type must be rejected");

    assert!(matches!(
        error,
        ConfigWireDecodeError::Json {
            category: Category::Data,
            line: 1,
            column: 15,
        }
    ));
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

    let mut wire = to_value(&config).expect("serializing config should succeed");
    wire["properties"]["server.item0"]["name"] = json!("bad.name");
    let input = to_vec(&wire).expect("serializing the wire object should succeed");
    let limits = ConfigWireLimits::default();
    let decode = JsonDecodeLimits::builder()
        .input_bytes_limit(ResourceLimit::new(
            JsonResource::InputBytes,
            u64::try_from(input.len()).expect("input length must fit"),
        ))
        .build();
    let limits = ConfigWireLimits::builder_from(&limits).json_decode(decode).build();

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

    let mut wire = to_value(&config).expect("serializing config should succeed");
    wire["future_field"] = json!(true);

    from_value::<Config>(wire).expect_err("unknown V1 fields must not silently deserialize as legacy");
}

/// Verifies legacy runtime policy data is not part of the versioned contract.
#[test]
fn test_config_wire_rejects_read_options_in_v1_payload() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = to_value(&config).expect("serializing config should succeed");
    wire["read_options"] = json!({"environment_fallback_enabled": true});

    from_value::<Config>(wire).expect_err("versioned config wire must not contain runtime policies");
}

/// Verifies persisted map keys cannot disagree with their embedded property
/// name.
#[test]
fn test_config_wire_rejects_property_name_mismatch() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = to_value(&config).expect("serializing config should succeed");
    wire["properties"]["server.port"]["name"] = json!("wrong.name");

    let error = from_value::<Config>(wire).expect_err("property name mismatches must be rejected");

    assert!(error.to_string().contains("does not match property name"));
}

#[test]
fn test_config_wire_rejects_duplicate_property_map_keys() {
    let input = br#"{"version":1,"properties":{"server.port":{"name":"server.port","value":{"version":1,"value":{"scalar":{"int32":1}}},"description":null,"is_final":false},"server.port":{"name":"server.port","value":{"version":1,"value":{"scalar":{"int32":2}}},"description":null,"is_final":false}}}"#;

    assert!(
        Config::decode_json_slice(input).is_err(),
        "duplicate persisted property keys must not be silently overwritten"
    );
}

#[test]
fn test_ordinary_deserialize_rejects_duplicate_property_map_keys() {
    let input = br#"{"version":1,"properties":{"server.port":{"name":"server.port","value":{"version":1,"value":{"scalar":{"int32":1}}},"description":null,"is_final":false},"server.port":{"name":"server.port","value":{"version":1,"value":{"scalar":{"int32":2}}},"description":null,"is_final":false}}}"#;

    let error = from_slice::<Config>(input)
        .expect_err("ordinary Deserialize must not silently overwrite duplicate property keys");
    assert!(
        error.to_string().contains("duplicate JSON object key"),
        "unexpected duplicate-key error: {error}"
    );
}

/// Verifies matching map/property names are still rejected when the common
/// name is not a canonical dotted key.
#[test]
fn test_config_wire_rejects_malformed_map_key() {
    let mut config = Config::new();
    config
        .set("server.port", 8080_u16)
        .expect("setting the property should succeed");

    let mut wire = to_value(&config).expect("serializing config should succeed");
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

    let error = from_value::<Config>(wire).expect_err("malformed config wire keys must be rejected");

    assert!(
        error.to_string().contains("bad..key"),
        "unexpected deserialization error: {error}"
    );
}
