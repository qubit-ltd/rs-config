// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Configuration-specific profiles layered on rs-budget JSON limits.
// qubit-style: allow multiple-public-types

use qubit_budget::BudgetError;
use qubit_budget::JsonLimits;
use qubit_budget::JsonResource;
use thiserror::Error;

/// Resource categories specific to configuration wire envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigWireLimitKind {
    /// Number of persisted properties.
    Properties,
    /// UTF-8 bytes in one property key.
    PropertyKeyBytes,
}

/// Limits applied while decoding or encoding a complete configuration wire
/// document.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigWireLimits {
    json: JsonLimits,
    max_properties: usize,
    max_property_key_bytes: usize,
}

impl ConfigWireLimits {
    /// Default maximum persisted properties.
    pub const DEFAULT_MAX_PROPERTIES: usize = 4_096;
    /// Default maximum UTF-8 bytes in one property key.
    pub const DEFAULT_MAX_PROPERTY_KEY_BYTES: usize = 256;

    /// Creates configuration limits with the specified input-byte bound.
    #[inline(always)]
    pub const fn new(max_input_bytes: usize) -> Self {
        Self {
            json: Self::default_json_limits(max_input_bytes),
            max_properties: Self::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes: Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Creates configuration limits from an already configured JSON limit set.
    #[inline(always)]
    pub const fn from_json(json: JsonLimits) -> Self {
        Self {
            json,
            max_properties: Self::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes: Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Replaces the shared JSON limits used by this configuration profile.
    #[inline(always)]
    #[must_use = "the configured JSON limits should be used"]
    pub const fn with_json(mut self, json: JsonLimits) -> Self {
        self.json = json;
        self
    }

    /// Sets the maximum number of persisted properties.
    #[inline(always)]
    #[must_use = "the configured property limit should be used"]
    pub const fn with_max_properties(mut self, max_properties: usize) -> Self {
        self.max_properties = max_properties;
        self
    }

    /// Sets the maximum UTF-8 bytes in one property key.
    #[inline(always)]
    #[must_use = "the configured property-key limit should be used"]
    pub const fn with_max_property_key_bytes(
        mut self,
        max_property_key_bytes: usize,
    ) -> Self {
        self.max_property_key_bytes = max_property_key_bytes;
        self
    }

    /// Returns the shared JSON limits.
    #[inline(always)]
    pub const fn json(self) -> JsonLimits {
        self.json
    }

    /// Returns the maximum number of persisted properties.
    #[must_use]
    #[inline(always)]
    pub const fn max_properties(self) -> usize {
        self.max_properties
    }

    /// Returns the maximum UTF-8 bytes in one property key.
    #[must_use]
    #[inline(always)]
    pub const fn max_property_key_bytes(self) -> usize {
        self.max_property_key_bytes
    }
}

impl Default for ConfigWireLimits {
    #[inline(always)]
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_INPUT_BYTES)
    }
}

impl ConfigWireLimits {
    /// Default maximum complete JSON input length.
    pub const DEFAULT_MAX_INPUT_BYTES: usize = 1_048_576;
    /// Default maximum complete JSON output length.
    pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1_048_576;
    /// Default maximum recursive JSON depth.
    pub const DEFAULT_MAX_DEPTH: usize = 64;
    /// Default maximum number of JSON nodes.
    pub const DEFAULT_MAX_NODES: usize = 100_000;
    /// Default maximum items in one JSON sequence.
    pub const DEFAULT_MAX_SEQUENCE_ITEMS: usize = 4_096;
    /// Default maximum entries in one JSON map.
    pub const DEFAULT_MAX_MAP_ENTRIES: usize = 4_096;
    /// Default maximum bytes in one JSON string.
    pub const DEFAULT_MAX_STRING_BYTES: usize = 256 * 1024;
    /// Default maximum bytes in one JSON number representation.
    pub const DEFAULT_MAX_NUMBER_BYTES: usize = 4_096;
    /// Default maximum bytes in one JSON object key.
    pub const DEFAULT_MAX_KEY_BYTES: usize = 256 * 1024;

    /// Builds the rs-budget profile used by the default configuration wire.
    const fn default_json_limits(max_input_bytes: usize) -> JsonLimits {
        JsonLimits::new()
            .with_max_input_bytes(max_input_bytes)
            .with_max_output_bytes(max_input_bytes)
            .with_max_depth(Self::DEFAULT_MAX_DEPTH)
            .with_max_nodes(Self::DEFAULT_MAX_NODES)
            .with_max_sequence_items(Self::DEFAULT_MAX_SEQUENCE_ITEMS)
            .with_max_map_entries(Self::DEFAULT_MAX_MAP_ENTRIES)
            .with_max_key_bytes(Self::DEFAULT_MAX_KEY_BYTES)
            .with_max_string_bytes(Self::DEFAULT_MAX_STRING_BYTES)
            .with_max_number_bytes(Self::DEFAULT_MAX_NUMBER_BYTES)
    }
}

/// Error returned by bounded configuration wire decoding.
#[derive(Debug, Error)]
pub enum ConfigWireDecodeError {
    /// A shared JSON resource limit was exceeded.
    #[error(transparent)]
    Budget(BudgetError<JsonResource, usize>),
    /// A configuration-specific resource limit was exceeded.
    #[error(
        "configuration wire {kind:?} value {value} exceeds the limit of {maximum}"
    )]
    LimitExceeded {
        /// Resource category that exceeded its limit.
        kind: ConfigWireLimitKind,
        /// Observed resource value.
        value: usize,
        /// Largest permitted resource value.
        maximum: usize,
    },
    /// The JSON document is malformed or violates its Serde representation.
    #[error("failed to decode configuration JSON wire input: {0}")]
    Json(#[source] serde_json::Error),
    /// The decoded wire fields violate a configuration invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}
