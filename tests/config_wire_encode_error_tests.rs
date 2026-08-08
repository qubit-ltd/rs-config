// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for bounded configuration wire encoding errors.

use qubit_config::ConfigWireDecodeError;
use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_value::ValueWireDecodeError;
use qubit_value::ValueWireLimitKind;

#[test]
fn config_wire_encode_error_maps_input_bytes_to_output_bytes() {
    let error = ConfigWireEncodeError::from(ConfigWireDecodeError::Value(
        ValueWireDecodeError::InputTooLarge {
            input_bytes: 17,
            max_input_bytes: 16,
        },
    ));

    assert!(matches!(
        error,
        ConfigWireEncodeError::OutputTooLarge {
            output_bytes: 17,
            max_output_bytes: 16,
        }
    ));
}

#[test]
fn config_wire_encode_error_preserves_shared_value_limit() {
    let error = ConfigWireEncodeError::from(ConfigWireDecodeError::Value(
        ValueWireDecodeError::LimitExceeded {
            kind: ValueWireLimitKind::Nodes,
            value: 9,
            maximum: 8,
        },
    ));

    assert!(matches!(
        error,
        ConfigWireEncodeError::ValueLimitExceeded {
            kind: ValueWireLimitKind::Nodes,
            value: 9,
            maximum: 8,
        }
    ));
}

#[test]
fn config_wire_encode_error_preserves_config_limit() {
    let error =
        ConfigWireEncodeError::from(ConfigWireDecodeError::LimitExceeded {
            kind: ConfigWireLimitKind::Properties,
            value: 5,
            maximum: 4,
        });

    assert!(matches!(
        error,
        ConfigWireEncodeError::LimitExceeded {
            kind: ConfigWireLimitKind::Properties,
            value: 5,
            maximum: 4,
        }
    ));
}
