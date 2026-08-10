// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for configuration wire resource limits.

use qubit_config::ConfigWireLimits;
use qubit_budget::JsonLimits;

#[test]
fn config_wire_limits_preserve_configured_shared_budget() {
    let json = JsonLimits::new()
        .with_max_input_bytes(123)
        .with_max_depth(9)
        .with_max_nodes(456);
    let limits = ConfigWireLimits::from_json(json)
        .with_max_properties(7)
        .with_max_property_key_bytes(8);

    assert_eq!(limits.json(), json);
    assert_eq!(limits.max_properties(), 7);
    assert_eq!(limits.max_property_key_bytes(), 8);
}
