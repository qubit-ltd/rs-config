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
use qubit_budget::JsonDecodeLimits;
use qubit_budget::JsonEncodeLimits;
use qubit_budget::JsonResource;
use qubit_budget::JsonValueLimits;
use qubit_budget::QuantityConversionError;
use qubit_budget::ResourceLimit;
use qubit_budget::StructureLimits;
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
    json_decode: JsonDecodeLimits,
    json_encode: JsonEncodeLimits,
    properties: ResourceLimit<ConfigWireLimitKind, u64>,
    property_key_bytes: ResourceLimit<ConfigWireLimitKind, u64>,
}

impl ConfigWireLimits {
    /// Default maximum persisted properties.
    pub const DEFAULT_MAX_PROPERTIES: u64 = 4_096;
    /// Default maximum UTF-8 bytes in one property key.
    pub const DEFAULT_MAX_PROPERTY_KEY_BYTES: u64 = 256;

    /// Creates configuration limits with the specified input-byte bound.
    #[inline(always)]
    pub fn new(max_input_bytes: u64) -> Self {
        Self {
            json_decode: Self::default_json_decode_limits(max_input_bytes),
            json_encode: Self::default_json_encode_limits(Self::DEFAULT_MAX_OUTPUT_BYTES),
            properties: ResourceLimit::new(
                ConfigWireLimitKind::Properties,
                Self::DEFAULT_MAX_PROPERTIES,
            ),
            property_key_bytes: ResourceLimit::new(
                ConfigWireLimitKind::PropertyKeyBytes,
                Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
            ),
        }
    }

    /// Creates configuration limits from an already configured JSON limit set.
    #[inline(always)]
    pub const fn from_json(json_decode: JsonDecodeLimits, json_encode: JsonEncodeLimits) -> Self {
        Self {
            json_decode,
            json_encode,
            properties: ResourceLimit::new(
                ConfigWireLimitKind::Properties,
                Self::DEFAULT_MAX_PROPERTIES,
            ),
            property_key_bytes: ResourceLimit::new(
                ConfigWireLimitKind::PropertyKeyBytes,
                Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
            ),
        }
    }

    /// Replaces the JSON decoding limits used by this configuration profile.
    #[inline(always)]
    #[must_use = "the configured JSON limits should be used"]
    pub const fn with_json_decode(mut self, json_decode: JsonDecodeLimits) -> Self {
        self.json_decode = json_decode;
        self
    }

    /// Replaces the JSON encoding limits used by this configuration profile.
    #[inline(always)]
    #[must_use = "the configured JSON limits should be used"]
    pub const fn with_json_encode(mut self, json_encode: JsonEncodeLimits) -> Self {
        self.json_encode = json_encode;
        self
    }

    /// Sets the maximum number of persisted properties.
    #[inline(always)]
    #[must_use = "the configured property limit should be used"]
    pub const fn with_max_properties(mut self, max_properties: u64) -> Self {
        self.properties = ResourceLimit::new(ConfigWireLimitKind::Properties, max_properties);
        self
    }

    /// Sets the maximum UTF-8 bytes in one property key.
    #[inline(always)]
    #[must_use = "the configured property-key limit should be used"]
    pub const fn with_max_property_key_bytes(mut self, max_property_key_bytes: u64) -> Self {
        self.property_key_bytes = ResourceLimit::new(
            ConfigWireLimitKind::PropertyKeyBytes,
            max_property_key_bytes,
        );
        self
    }

    /// Returns the JSON decoding limits.
    #[inline(always)]
    pub const fn json_decode(self) -> JsonDecodeLimits {
        self.json_decode
    }

    /// Returns the JSON encoding limits.
    #[inline(always)]
    pub const fn json_encode(self) -> JsonEncodeLimits {
        self.json_encode
    }

    /// Returns the maximum number of persisted properties.
    #[must_use]
    #[inline(always)]
    pub const fn max_properties(self) -> u64 {
        self.properties.maximum()
    }

    /// Returns the maximum UTF-8 bytes in one property key.
    #[must_use]
    #[inline(always)]
    pub const fn max_property_key_bytes(self) -> u64 {
        self.property_key_bytes.maximum()
    }

    /// Returns the complete property-count point limit.
    pub(crate) const fn properties_limit(&self) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.properties
    }

    /// Returns the complete property-key byte point limit.
    pub(crate) const fn property_key_bytes_limit(
        &self,
    ) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.property_key_bytes
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
    pub const DEFAULT_MAX_INPUT_BYTES: u64 = 1_048_576;
    /// Default maximum complete JSON output length.
    pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 1_048_576;
    /// Default maximum recursive JSON depth.
    pub const DEFAULT_MAX_DEPTH: u64 = 64;
    /// Default maximum number of JSON nodes.
    pub const DEFAULT_MAX_NODES: u64 = 100_000;
    /// Default maximum items in one JSON sequence.
    pub const DEFAULT_MAX_SEQUENCE_ITEMS: u64 = 4_096;
    /// Default maximum entries in one JSON map.
    pub const DEFAULT_MAX_MAP_ENTRIES: u64 = 4_096;
    /// Default maximum bytes in one JSON string.
    pub const DEFAULT_MAX_STRING_BYTES: u64 = 256 * 1024;
    /// Default maximum bytes in one JSON number representation.
    pub const DEFAULT_MAX_NUMBER_BYTES: u64 = 4_096;
    /// Default maximum bytes in one JSON object key.
    pub const DEFAULT_MAX_KEY_BYTES: u64 = 256 * 1024;
    /// Default cumulative bytes in JSON keys, strings, and numbers.
    pub const DEFAULT_MAX_PAYLOAD_BYTES: u64 = 1_048_576;

    /// Builds the direction-independent JSON value profile.
    fn default_json_value_limits() -> JsonValueLimits {
        let structure = StructureLimits::empty()
            .with_depth_limit(ResourceLimit::new(
                JsonResource::Depth,
                Self::DEFAULT_MAX_DEPTH,
            ))
            .with_nodes_limit(ResourceLimit::new(
                JsonResource::Nodes,
                Self::DEFAULT_MAX_NODES,
            ))
            .with_sequence_items_limit(ResourceLimit::new(
                JsonResource::SequenceItems,
                Self::DEFAULT_MAX_SEQUENCE_ITEMS,
            ))
            .with_map_entries_limit(ResourceLimit::new(
                JsonResource::MapEntries,
                Self::DEFAULT_MAX_MAP_ENTRIES,
            ))
            .with_key_bytes_limit(ResourceLimit::new(
                JsonResource::KeyBytes,
                Self::DEFAULT_MAX_KEY_BYTES,
            ));
        JsonValueLimits::default()
            .with_structure_limits(structure)
            .with_string_bytes_limit(ResourceLimit::new(
                JsonResource::StringBytes,
                Self::DEFAULT_MAX_STRING_BYTES,
            ))
            .with_number_bytes_limit(ResourceLimit::new(
                JsonResource::NumberBytes,
                Self::DEFAULT_MAX_NUMBER_BYTES,
            ))
            .with_payload_bytes_limit(ResourceLimit::new(
                JsonResource::PayloadBytes,
                Self::DEFAULT_MAX_PAYLOAD_BYTES,
            ))
    }

    /// Builds the JSON decoding profile used by configuration wire input.
    fn default_json_decode_limits(max_input_bytes: u64) -> JsonDecodeLimits {
        JsonDecodeLimits::default()
            .with_input_bytes_limit(ResourceLimit::new(
                JsonResource::InputBytes,
                max_input_bytes,
            ))
            .with_value_limits(Self::default_json_value_limits())
    }

    /// Builds the JSON encoding profile used by configuration wire output.
    fn default_json_encode_limits(max_output_bytes: u64) -> JsonEncodeLimits {
        JsonEncodeLimits::default()
            .with_output_bytes_limit(ResourceLimit::new(
                JsonResource::OutputBytes,
                max_output_bytes,
            ))
            .with_value_limits(Self::default_json_value_limits())
    }
}

/// Error returned by bounded configuration wire decoding.
#[derive(Debug, Error)]
pub enum ConfigWireDecodeError {
    /// A shared JSON resource limit was exceeded.
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
        /// Resource category that exceeded its limit.
        kind: ConfigWireLimitKind,
        /// Observed resource value.
        value: u64,
        /// Largest permitted resource value.
        maximum: u64,
    },
    /// The JSON document is malformed or violates its Serde representation.
    #[error("failed to decode configuration JSON wire input: {0}")]
    Json(#[source] serde_json::Error),
    /// The decoded wire fields violate a configuration invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}
