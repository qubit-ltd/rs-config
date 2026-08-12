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
use qubit_config::ConfigWireLimits;
use qubit_json::JsonDecodeLimits;
use qubit_json::JsonEncodeLimits;
use qubit_json::JsonResource;
use qubit_json::JsonValueLimits;

#[test]
fn config_wire_limits_preserve_configured_shared_budget() {
    let value = JsonValueLimits::default().with_structure_limits(
        StructureLimits::empty()
            .with_depth_limit(ResourceLimit::new(JsonResource::Depth, 9))
            .with_nodes_limit(ResourceLimit::new(JsonResource::Nodes, 456)),
    );
    let decode = JsonDecodeLimits::default()
        .with_input_bytes_limit(ResourceLimit::new(
            JsonResource::InputBytes,
            123,
        ))
        .with_value_limits(value);
    let encode = JsonEncodeLimits::default().with_value_limits(value);
    let limits = ConfigWireLimits::from_json(decode, encode)
        .with_max_properties(7)
        .with_max_property_key_bytes(8);

    assert_eq!(limits.json_decode(), decode);
    assert_eq!(limits.json_encode(), encode);
    assert_eq!(limits.max_properties(), 7);
    assert_eq!(limits.max_property_key_bytes(), 8);
}
