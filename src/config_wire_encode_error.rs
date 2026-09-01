// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors produced by bounded configuration JSON wire encoding.

use qubit_budget::BudgetError;
use qubit_budget::MeasuredBudgetError;
use qubit_budget::QuantityConversionError;
use qubit_budget::json::JsonResource;
use qubit_json::decode::JsonSyntaxError;
use qubit_json::encode::JsonEncodeError;
use qubit_json::encode::JsonEncodeErrorKind;
use qubit_json::encode::JsonSerializationError;
use qubit_value::ValueWireEncodeError;
use thiserror::Error;

use crate::ConfigWireLimitKind;

/// Error returned by bounded configuration JSON wire encoding.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigWireEncodeError {
    /// A runtime value cannot be represented by the V1 JSON wire format.
    #[error(transparent)]
    Value(#[from] ValueWireEncodeError),

    /// A shared JSON resource limit was exceeded during serialization.
    #[error(transparent)]
    Budget(BudgetError<JsonResource, u64>),
    /// A native JSON measurement could not be represented by the budget
    /// quantity type.
    #[error(
        "configuration wire resource quantity conversion failed for {resource:?}: {source}"
    )]
    Quantity {
        /// Resource whose measurement failed.
        resource: JsonResource,
        /// Native measurement conversion failure.
        #[source]
        source: QuantityConversionError,
    },

    /// A native configuration-limit measurement could not fit `u64`.
    #[error("configuration wire {kind:?} quantity conversion failed: {source}")]
    LimitQuantity {
        /// Configuration-specific resource category being measured.
        kind: ConfigWireLimitKind,
        /// Exact failed native quantity conversion.
        #[source]
        source: QuantityConversionError,
    },

    /// A configuration-specific resource limit was exceeded.
    #[error(
        "configuration wire {kind:?} value {value} exceeds the limit of {maximum}"
    )]
    LimitExceeded {
        /// Configuration resource category that exceeded its limit.
        kind: ConfigWireLimitKind,

        /// Observed resource value.
        value: u64,

        /// Largest permitted resource value.
        maximum: u64,
    },

    /// The JSON adapter rejected invalid raw JSON syntax during serialization.
    #[error("invalid configuration JSON syntax: {0}")]
    Syntax(#[from] JsonSyntaxError),

    /// Strict JSON serialization failed after bounded preflight validation.
    #[error("failed to encode configuration JSON wire output: {0}")]
    Json(#[from] JsonSerializationError),

    /// A future JSON adapter failure that has no dedicated configuration
    /// error variant yet.
    #[error("configuration JSON adapter failure: {0}")]
    Adapter(String),

    /// The runtime configuration violates a persisted wire invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}

impl From<JsonEncodeError<JsonResource, u64>> for ConfigWireEncodeError {
    /// Preserves the exact resource identity of a bounded JSON encoding
    /// failure.
    #[inline]
    fn from(error: JsonEncodeError<JsonResource, u64>) -> Self {
        match error.kind() {
            JsonEncodeErrorKind::Budget => match error
                .into_budget_error()
                .expect("budget kind must retain a budget source")
            {
                MeasuredBudgetError::Budget(error) => Self::Budget(error),
                MeasuredBudgetError::Quantity { resource, source } => {
                    Self::Quantity { resource, source }
                }
            },
            JsonEncodeErrorKind::InvalidRawJson => {
                Self::Syntax(error.into_syntax_error().expect(
                    "invalid raw JSON kind must retain a syntax source",
                ))
            }
            JsonEncodeErrorKind::Serialize => {
                Self::Json(error.into_serialization_error().expect(
                    "serialize kind must retain a serialization source",
                ))
            }
            JsonEncodeErrorKind::Write => Self::Adapter(String::from(
                "unexpected writer failure while buffering configuration JSON",
            )),
        }
    }
}
