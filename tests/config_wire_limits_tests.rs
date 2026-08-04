// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
// =============================================================================

//! Tests for configuration wire resource limits.

use qubit_config::ConfigWireLimits;
use qubit_value::WireLimits;

#[test]
fn config_wire_limits_preserve_configured_shared_budget() {
    let wire = WireLimits::new(123).with_max_depth(9).with_max_nodes(456);
    let limits = ConfigWireLimits::from_wire(wire)
        .with_max_properties(7)
        .with_max_property_key_bytes(8);

    assert_eq!(limits.wire(), wire);
    assert_eq!(limits.max_properties(), 7);
    assert_eq!(limits.max_property_key_bytes(), 8);
}
