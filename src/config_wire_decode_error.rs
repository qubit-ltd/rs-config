// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_budget::BudgetError;
use qubit_budget::QuantityConversionError;
use qubit_budget::json::JsonResource;
use qubit_json::decode::JsonSyntaxError;
use serde_json::error::Category;
use thiserror::Error;

use super::ConfigWireLimitKind;
use crate::ConfigWireRepresentationErrorKind;

/// Error returned by bounded configuration wire decoding.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigWireDecodeError, ConfigWireLimitKind, ConfigWireLimits};
///
/// let mut config = Config::new();
/// config.set("port", 8080_u16)?;
/// let bytes = config.encode_json_vec()?;
/// let limits = ConfigWireLimits::builder().max_properties(0).build();
/// match Config::decode_json_slice_with_limits(&bytes, limits) {
///     Err(ConfigWireDecodeError::LimitExceeded {
///         kind: ConfigWireLimitKind::Properties,
///         ..
///     }) => {}
///     other => panic!("expected property limit, got {other:?}"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Error)]
#[must_use]
#[non_exhaustive]
pub enum ConfigWireDecodeError {
    /// A shared JSON resource limit was exceeded.
    #[error(transparent)]
    Budget(
        /// Shared JSON resource budget failure.
        BudgetError<JsonResource, u64>,
    ),
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
    /// The JSON adapter rejected the input syntax before Serde decoding.
    #[error("invalid configuration JSON syntax: {0}")]
    Syntax(
        /// Syntax failure returned by the JSON adapter.
        #[from]
        JsonSyntaxError,
    ),
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
    #[error("configuration wire {kind:?} value {value} exceeds the limit of {maximum}")]
    LimitExceeded {
        /// Resource category that exceeded its limit.
        kind: ConfigWireLimitKind,
        /// Observed resource value.
        value: u64,
        /// Largest permitted resource value.
        maximum: u64,
    },
    /// The admitted JSON document violates its Serde representation.
    #[error("failed to decode configuration JSON wire input ({category:?}) at line {line}, column {column}")]
    Json {
        /// Broad Serde failure category.
        category: Category,
        /// One-based line reported by Serde, or zero when unavailable.
        line: usize,
        /// One-based column reported by Serde, or zero when unavailable.
        column: usize,
    },
    /// The admitted JSON document does not match the persisted wire shape.
    #[error("invalid configuration wire representation at '{path}': {reason:?}")]
    Representation {
        /// JSON Pointer to the invalid wire location.
        path: String,
        /// Safe classification of the representation failure.
        reason: ConfigWireRepresentationErrorKind,
    },
    /// A future JSON adapter failure that has no dedicated configuration
    /// error variant yet.
    #[error("configuration JSON adapter failure: {0}")]
    Adapter(
        /// Safe adapter failure description.
        String,
    ),
    /// The decoded wire fields violate a configuration invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(
        /// Description of the violated configuration invariant.
        String,
    ),
}
