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
