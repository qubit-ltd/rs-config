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
