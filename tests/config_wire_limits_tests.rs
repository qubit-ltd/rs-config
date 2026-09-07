// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for configuration wire resource limits.

use qubit_budget::ResourceLimit;
use qubit_budget::StructureLimits;
use qubit_budget::json::JsonDecodeLimits;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueLimits;
use qubit_config::ConfigWireLimits;
use qubit_config::ConfigWireLimitsBuilder;

#[test]
fn config_wire_limits_preserve_configured_shared_budget() {
    let value = JsonValueLimits::builder()
        .structure_limits(
            StructureLimits::<JsonResource, u64>::builder()
                .depth_limit(ResourceLimit::new(JsonResource::Depth, 9))
                .nodes_limit(ResourceLimit::new(JsonResource::Nodes, 456))
                .build(),
        )
        .build();
    let decode = JsonDecodeLimits::builder()
        .input_bytes_limit(ResourceLimit::new(JsonResource::InputBytes, 123))
        .value_limits(value)
        .build();
    let encode = JsonEncodeLimits::builder().value_limits(value).build();
    let limits = ConfigWireLimits::builder()
        .json_decode(decode)
        .json_encode(encode)
        .max_properties(7)
        .max_property_key_bytes(8)
        .build();

    assert_eq!(limits.json_decode(), decode);
    assert_eq!(limits.json_encode(), encode);
    assert_eq!(limits.max_properties(), 7);
    assert_eq!(limits.max_property_key_bytes(), 8);
}

#[test]
fn config_wire_limits_support_json_profiles_and_default_builder_overrides() {
    let decode = JsonDecodeLimits::builder().max_input_bytes(17).build();
    let encode = JsonEncodeLimits::builder().max_output_bytes(19).build();
    let from_json = ConfigWireLimits::from_json(decode, encode);

    assert_eq!(from_json.json_decode(), decode);
    assert_eq!(from_json.json_encode(), encode);
    assert_eq!(
        from_json.max_properties(),
        ConfigWireLimits::DEFAULT_MAX_PROPERTIES
    );
    assert_eq!(
        from_json.max_property_key_bytes(),
        ConfigWireLimits::DEFAULT_MAX_PROPERTY_KEY_BYTES
    );

    let overridden = ConfigWireLimitsBuilder::default()
        .max_input_bytes(23)
        .build();
    assert_eq!(overridden.json_decode().max_input_bytes(), Some(23));
}

#[test]
fn config_wire_limits_builder_covers_every_json_budget_dimension() {
    let decode = JsonDecodeLimits::builder()
        .max_input_bytes(11)
        .max_depth(12)
        .max_nodes(13)
        .max_sequence_items(14)
        .max_map_entries(15)
        .max_key_bytes(16)
        .max_string_bytes(17)
        .max_number_bytes(18)
        .max_payload_bytes(19)
        .build();
    let encode = JsonEncodeLimits::builder()
        .max_output_bytes(21)
        .max_depth(22)
        .max_nodes(23)
        .max_sequence_items(24)
        .max_map_entries(25)
        .max_key_bytes(26)
        .max_string_bytes(27)
        .max_number_bytes(28)
        .max_payload_bytes(29)
        .build();
    let limits = ConfigWireLimits::builder()
        .json_decode(decode)
        .json_encode(encode)
        .max_properties(31)
        .max_property_key_bytes(32)
        .build();

    assert_eq!(limits.max_properties(), 31);
    assert_eq!(limits.max_property_key_bytes(), 32);
    assert_eq!(limits.json_decode().max_input_bytes(), Some(11));
    assert_eq!(limits.json_decode().value_limits().max_depth(), Some(12));
    assert_eq!(limits.json_decode().value_limits().max_nodes(), Some(13));
    assert_eq!(
        limits.json_decode().value_limits().max_sequence_items(),
        Some(14)
    );
    assert_eq!(
        limits.json_decode().value_limits().max_map_entries(),
        Some(15)
    );
    assert_eq!(
        limits.json_decode().value_limits().max_key_bytes(),
        Some(16)
    );
    assert_eq!(
        limits.json_decode().value_limits().max_string_bytes(),
        Some(17)
    );
    assert_eq!(
        limits.json_decode().value_limits().max_number_bytes(),
        Some(18)
    );
    assert_eq!(
        limits.json_decode().value_limits().max_payload_bytes(),
        Some(19)
    );
    assert_eq!(limits.json_encode().max_output_bytes(), Some(21));
    assert_eq!(limits.json_encode().value_limits().max_depth(), Some(22));
    assert_eq!(limits.json_encode().value_limits().max_nodes(), Some(23));
    assert_eq!(
        limits.json_encode().value_limits().max_sequence_items(),
        Some(24)
    );
    assert_eq!(
        limits.json_encode().value_limits().max_map_entries(),
        Some(25)
    );
    assert_eq!(
        limits.json_encode().value_limits().max_key_bytes(),
        Some(26)
    );
    assert_eq!(
        limits.json_encode().value_limits().max_string_bytes(),
        Some(27)
    );
    assert_eq!(
        limits.json_encode().value_limits().max_number_bytes(),
        Some(28)
    );
    assert_eq!(
        limits.json_encode().value_limits().max_payload_bytes(),
        Some(29)
    );
}

#[test]
fn config_wire_scalar_limit_getters_are_callable_as_functions() {
    let limits = ConfigWireLimits::builder()
        .max_properties(11)
        .max_property_key_bytes(12)
        .build();
    let max_properties: fn(ConfigWireLimits) -> u64 = ConfigWireLimits::max_properties;
    let max_property_key_bytes: fn(ConfigWireLimits) -> u64 = ConfigWireLimits::max_property_key_bytes;

    assert_eq!(std::hint::black_box(max_properties)(limits), 11);
    assert_eq!(std::hint::black_box(max_property_key_bytes)(limits), 12);
}
