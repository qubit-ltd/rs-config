// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for bounded configuration wire encoding errors.

use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_budget::BudgetError;
use qubit_budget::JsonResource;

/// Verifies shared budget errors are exposed without a lossy conversion.
#[test]
fn config_wire_encode_error_exposes_budget_source() {
    let error = ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
        resource: JsonResource::OutputBytes,
        actual: 17,
        maximum: 16,
    });

    assert!(matches!(
        error,
        ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::OutputBytes,
            actual: 17,
            maximum: 16,
        }
    ));
}

/// Verifies the resource identity remains available to callers.
#[test]
fn config_wire_encode_error_preserves_budget_resource() {
    let error = ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
        resource: JsonResource::Nodes,
        actual: 9,
        maximum: 8,
    });

    assert!(matches!(
        error,
        ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::Nodes,
            actual: 9,
            maximum: 8,
        }
    ));
}

/// Verifies configuration-specific limits remain distinct from JSON budgets.
#[test]
fn config_wire_encode_error_preserves_config_limit() {
    let error = ConfigWireEncodeError::LimitExceeded {
        kind: ConfigWireLimitKind::Properties,
        value: 5,
        maximum: 4,
    };

    assert!(matches!(
        error,
        ConfigWireEncodeError::LimitExceeded {
            kind: ConfigWireLimitKind::Properties,
            value: 5,
            maximum: 4,
        }
    ));
}
