// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors produced by bounded configuration JSON wire encoding.

use qubit_value::ValueWireDecodeError;
use qubit_value::ValueWireEncodeError;
use qubit_value::ValueWireLimitKind;
use thiserror::Error;

use crate::ConfigWireDecodeError;
use crate::ConfigWireLimitKind;

/// Error returned by bounded configuration JSON wire encoding.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigWireEncodeError {
    /// A runtime value cannot be represented by the V1 JSON wire format.
    #[error(transparent)]
    Value(#[from] ValueWireEncodeError),

    /// A shared value resource limit was exceeded before serialization.
    #[error(
        "configuration wire {kind:?} value {value} exceeds the limit of {maximum}"
    )]
    ValueLimitExceeded {
        /// Shared value resource category that exceeded its limit.
        kind: ValueWireLimitKind,

        /// Observed resource value.
        value: usize,

        /// Largest permitted resource value.
        maximum: usize,
    },

    /// A shared wire error added by a newer value-wire implementation.
    #[error(transparent)]
    Shared(ValueWireDecodeError),

    /// A configuration-specific resource limit was exceeded.
    #[error(
        "configuration wire {kind:?} value {value} exceeds the limit of {maximum}"
    )]
    LimitExceeded {
        /// Configuration resource category that exceeded its limit.
        kind: ConfigWireLimitKind,

        /// Observed resource value.
        value: usize,

        /// Largest permitted resource value.
        maximum: usize,
    },

    /// The serialized JSON document exceeds the configured byte limit.
    #[error(
        "configuration wire output contains {output_bytes} bytes, exceeding the {max_output_bytes}-byte limit"
    )]
    OutputTooLarge {
        /// Complete serialized output length.
        output_bytes: usize,

        /// Maximum permitted serialized output length.
        max_output_bytes: usize,
    },

    /// JSON serialization failed after bounded preflight validation.
    #[error("failed to encode configuration JSON wire output: {0}")]
    Json(#[from] serde_json::Error),

    /// The runtime configuration violates a persisted wire invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}

impl From<ConfigWireDecodeError> for ConfigWireEncodeError {
    /// Converts shared budget failures into encoding-specific diagnostics.
    fn from(error: ConfigWireDecodeError) -> Self {
        match error {
            ConfigWireDecodeError::Value(
                ValueWireDecodeError::InputTooLarge {
                    input_bytes,
                    max_input_bytes,
                },
            ) => Self::OutputTooLarge {
                output_bytes: input_bytes,
                max_output_bytes: max_input_bytes,
            },
            ConfigWireDecodeError::Value(
                ValueWireDecodeError::LimitExceeded {
                    kind,
                    value,
                    maximum,
                },
            ) => Self::ValueLimitExceeded {
                kind,
                value,
                maximum,
            },
            ConfigWireDecodeError::Value(
                ValueWireDecodeError::InvalidJson(error),
            )
            | ConfigWireDecodeError::InvalidJson(error) => Self::Json(error),
            ConfigWireDecodeError::Value(error) => Self::Shared(error),
            ConfigWireDecodeError::LimitExceeded {
                kind,
                value,
                maximum,
            } => Self::LimitExceeded {
                kind,
                value,
                maximum,
            },
            ConfigWireDecodeError::InvalidConfig(message) => {
                Self::InvalidConfig(message)
            }
        }
    }
}
