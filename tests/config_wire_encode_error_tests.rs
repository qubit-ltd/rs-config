// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for bounded configuration wire encoding errors.

use std::io;
use std::io::Write;

use qubit_budget::BudgetError;
use qubit_budget::MeasuredBudgetError;
use qubit_budget::Observation;
use qubit_budget::QuantityConversionError;
use qubit_budget::QuantityMeasurement;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;
use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_json::encode::JsonEncodeError;
use qubit_json::encode::JsonEncoder;
use qubit_json::encode::JsonIntegerSignedness;
use qubit_json::encode::JsonSerializationErrorKind;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeStruct;

/// Emits invalid JSON through serde_json's public RawValue protocol behavior.
struct InvalidRawValue;

impl Serialize for InvalidRawValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let token = concat!("$", "serde_json", ":", ":private::RawValue");
        let mut state = serializer.serialize_struct(token, 1)?;
        state.serialize_field(token, "[")?;
        state.end()
    }
}

/// Rejects every non-empty JSON output write.
struct RejectingWriter;

impl Write for RejectingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("rejected"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Verifies shared budget errors are exposed without a lossy conversion.
#[test]
fn config_wire_encode_error_exposes_budget_source() {
    let error = ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
        resource: JsonResource::OutputBytes,
        observed: Observation::Exact(17),
        maximum: 16,
    });

    assert!(matches!(
        error,
        ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::OutputBytes,
            observed: Observation::Exact(17),
            maximum: 16,
        })
    ));
}

/// Verifies configuration conversion retains structured serialization errors.
#[test]
fn config_wire_encode_error_preserves_serialization_kind() {
    let mut encoder = JsonEncoder::with_limits(JsonEncodeLimits::<
        JsonResource,
        u64,
    >::default());
    let source = encoder
        .to_vec(&u128::MAX)
        .expect_err("wide integer must fail JSON serialization");
    let error = ConfigWireEncodeError::from(source);

    assert!(matches!(
        error,
        ConfigWireEncodeError::Json(source)
            if source.kind() == JsonSerializationErrorKind::IntegerOutOfRange {
                signedness: JsonIntegerSignedness::Unsigned,
            }
    ));
}

/// Verifies syntax and buffered-writer sources use their dedicated
/// configuration mapping policies.
#[test]
fn config_wire_encode_error_maps_syntax_and_writer_sources() {
    let mut raw_encoder = JsonEncoder::with_limits(JsonEncodeLimits::<
        JsonResource,
        u64,
    >::default());
    let syntax = raw_encoder
        .to_vec(&InvalidRawValue)
        .expect_err("invalid RawValue text must fail");
    assert!(matches!(
        ConfigWireEncodeError::from(syntax),
        ConfigWireEncodeError::Syntax(_)
    ));

    let mut writer_encoder = JsonEncoder::with_limits(JsonEncodeLimits::<
        JsonResource,
        u64,
    >::default());
    let writer = writer_encoder
        .write_buffered(RejectingWriter, &true)
        .expect_err("rejecting writer must fail");
    assert!(matches!(
        ConfigWireEncodeError::from(writer),
        ConfigWireEncodeError::Adapter(message)
            if message == "unexpected writer failure while buffering configuration JSON"
    ));
}

/// Verifies the resource identity remains available to callers.
#[test]
fn config_wire_encode_error_preserves_budget_resource() {
    let error = ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
        resource: JsonResource::Nodes,
        observed: Observation::Exact(9),
        maximum: 8,
    });

    assert!(matches!(
        error,
        ConfigWireEncodeError::Budget(BudgetError::LimitExceeded {
            resource: JsonResource::Nodes,
            observed: Observation::Exact(9),
            maximum: 8,
        })
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

/// Verifies native JSON quantity failures remain distinct from budget limits.
#[test]
fn config_wire_encode_error_preserves_json_quantity_failure() {
    let source = QuantityConversionError::new(
        QuantityMeasurement::Usize(usize::MAX),
        "u64",
    );
    let error = ConfigWireEncodeError::from(JsonEncodeError::from(
        MeasuredBudgetError::Quantity {
            resource: JsonResource::OutputBytes,
            source,
        },
    ));

    assert!(matches!(
        error,
        ConfigWireEncodeError::Quantity {
            resource: JsonResource::OutputBytes,
            source,
        } if source == QuantityConversionError::new(
            QuantityMeasurement::Usize(usize::MAX),
            "u64",
        )
    ));
}
