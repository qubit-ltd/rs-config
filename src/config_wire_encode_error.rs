// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors produced by bounded configuration JSON wire encoding.

use qubit_budget::BudgetError;
use qubit_budget::JsonResource;
use qubit_budget::QuantityConversionError;
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
    #[error("configuration wire resource quantity conversion failed for {resource:?}: {source}")]
    Quantity {
        /// Resource whose measurement failed.
        resource: JsonResource,
        /// Native measurement conversion failure.
        #[source]
        source: QuantityConversionError,
    },

    /// A configuration-specific resource limit was exceeded.
    #[error("configuration wire {kind:?} value {value} exceeds the limit of {maximum}")]
    LimitExceeded {
        /// Configuration resource category that exceeded its limit.
        kind: ConfigWireLimitKind,

        /// Observed resource value.
        value: u64,

        /// Largest permitted resource value.
        maximum: u64,
    },

    /// JSON serialization failed after bounded preflight validation.
    #[error("failed to encode configuration JSON wire output: {0}")]
    Json(#[from] serde_json::Error),

    /// The runtime configuration violates a persisted wire invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}
