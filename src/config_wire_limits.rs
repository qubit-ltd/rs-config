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
use qubit_budget::QuantityConversionError;
use qubit_budget::ResourceLimit;
use qubit_budget::json::JsonDecodeLimits;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueLimits;
use qubit_json::decode::JsonSyntaxError;
use serde_json::error::Category;
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
    json_decode: JsonDecodeLimits<JsonResource, u64>,
    json_encode: JsonEncodeLimits<JsonResource, u64>,
    properties: ResourceLimit<ConfigWireLimitKind, u64>,
    property_key_bytes: ResourceLimit<ConfigWireLimitKind, u64>,
}

impl ConfigWireLimits {
    /// Default maximum persisted properties.
    pub const DEFAULT_MAX_PROPERTIES: u64 = 4_096;
    /// Default maximum UTF-8 bytes in one property key.
    pub const DEFAULT_MAX_PROPERTY_KEY_BYTES: u64 = 256;

    /// Creates a builder initialized with the default wire limits.
    #[inline]
    pub fn builder() -> ConfigWireLimitsBuilder {
        ConfigWireLimitsBuilder::new()
    }

    /// Creates a builder initialized from existing wire limits.
    #[inline]
    pub fn builder_from(limits: &Self) -> ConfigWireLimitsBuilder {
        ConfigWireLimitsBuilder {
            json_decode: limits.json_decode,
            json_encode: limits.json_encode,
            max_properties: limits.properties.maximum(),
            max_property_key_bytes: limits.property_key_bytes.maximum(),
        }
    }

    /// Creates configuration limits from an already configured JSON limit set.
    #[inline(always)]
    pub const fn from_json(
        json_decode: JsonDecodeLimits<JsonResource, u64>,
        json_encode: JsonEncodeLimits<JsonResource, u64>,
    ) -> Self {
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

    /// Returns the JSON decoding limits.
    #[inline(always)]
    pub const fn json_decode(self) -> JsonDecodeLimits<JsonResource, u64> {
        self.json_decode
    }

    /// Returns the JSON encoding limits.
    #[inline(always)]
    pub const fn json_encode(self) -> JsonEncodeLimits<JsonResource, u64> {
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
    pub(crate) const fn properties_limit(
        &self,
    ) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.properties
    }

    /// Returns the complete property-key byte point limit.
    pub(crate) const fn property_key_bytes_limit(
        &self,
    ) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.property_key_bytes
    }
}

/// Builder for [`ConfigWireLimits`].
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigWireLimitsBuilder {
    json_decode: JsonDecodeLimits<JsonResource, u64>,
    json_encode: JsonEncodeLimits<JsonResource, u64>,
    max_properties: u64,
    max_property_key_bytes: u64,
}

impl ConfigWireLimitsBuilder {
    /// Creates a builder initialized with the default wire limits.
    #[inline]
    pub fn new() -> Self {
        Self {
            json_decode: ConfigWireLimits::default_json_decode_limits(
                ConfigWireLimits::DEFAULT_MAX_INPUT_BYTES,
            ),
            json_encode: ConfigWireLimits::default_json_encode_limits(
                ConfigWireLimits::DEFAULT_MAX_OUTPUT_BYTES,
            ),
            max_properties: ConfigWireLimits::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes:
                ConfigWireLimits::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Sets the maximum complete JSON input length.
    #[inline]
    pub fn max_input_bytes(mut self, maximum: u64) -> Self {
        self.json_decode =
            ConfigWireLimits::default_json_decode_limits(maximum);
        self
    }

    /// Replaces the JSON decoding limits.
    #[inline]
    pub fn json_decode(
        mut self,
        limits: JsonDecodeLimits<JsonResource, u64>,
    ) -> Self {
        self.json_decode = limits;
        self
    }

    /// Replaces the JSON encoding limits.
    #[inline]
    pub fn json_encode(
        mut self,
        limits: JsonEncodeLimits<JsonResource, u64>,
    ) -> Self {
        self.json_encode = limits;
        self
    }

    /// Sets the maximum number of persisted properties.
    #[inline]
    pub const fn max_properties(mut self, maximum: u64) -> Self {
        self.max_properties = maximum;
        self
    }

    /// Sets the maximum UTF-8 bytes in one property key.
    #[inline]
    pub const fn max_property_key_bytes(mut self, maximum: u64) -> Self {
        self.max_property_key_bytes = maximum;
        self
    }

    /// Builds the configured wire limits.
    #[inline]
    pub const fn build(self) -> ConfigWireLimits {
        ConfigWireLimits {
            json_decode: self.json_decode,
            json_encode: self.json_encode,
            properties: ResourceLimit::new(
                ConfigWireLimitKind::Properties,
                self.max_properties,
            ),
            property_key_bytes: ResourceLimit::new(
                ConfigWireLimitKind::PropertyKeyBytes,
                self.max_property_key_bytes,
            ),
        }
    }
}

impl Default for ConfigWireLimitsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ConfigWireLimits {
    #[inline(always)]
    fn default() -> Self {
        Self::builder().build()
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
    fn default_json_value_limits() -> JsonValueLimits<JsonResource, u64> {
        JsonValueLimits::<JsonResource, u64>::builder()
            .max_depth(Self::DEFAULT_MAX_DEPTH)
            .max_nodes(Self::DEFAULT_MAX_NODES)
            .max_sequence_items(Self::DEFAULT_MAX_SEQUENCE_ITEMS)
            .max_map_entries(Self::DEFAULT_MAX_MAP_ENTRIES)
            .max_key_bytes(Self::DEFAULT_MAX_KEY_BYTES)
            .max_string_bytes(Self::DEFAULT_MAX_STRING_BYTES)
            .max_number_bytes(Self::DEFAULT_MAX_NUMBER_BYTES)
            .max_payload_bytes(Self::DEFAULT_MAX_PAYLOAD_BYTES)
            .build()
    }

    /// Builds the JSON decoding profile used by configuration wire input.
    fn default_json_decode_limits(
        max_input_bytes: u64,
    ) -> JsonDecodeLimits<JsonResource, u64> {
        JsonDecodeLimits::builder()
            .max_input_bytes(max_input_bytes)
            .value_limits(Self::default_json_value_limits())
            .build()
    }

    /// Builds the JSON encoding profile used by configuration wire output.
    fn default_json_encode_limits(
        max_output_bytes: u64,
    ) -> JsonEncodeLimits<JsonResource, u64> {
        JsonEncodeLimits::builder()
            .max_output_bytes(max_output_bytes)
            .value_limits(Self::default_json_value_limits())
            .build()
    }
}

/// Error returned by bounded configuration wire decoding.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigWireDecodeError {
    /// A shared JSON resource limit was exceeded.
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
    /// The JSON adapter rejected the input syntax before Serde decoding.
    #[error("invalid configuration JSON syntax: {0}")]
    Syntax(#[from] JsonSyntaxError),
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
        /// Resource category that exceeded its limit.
        kind: ConfigWireLimitKind,
        /// Observed resource value.
        value: u64,
        /// Largest permitted resource value.
        maximum: u64,
    },
    /// The admitted JSON document violates its Serde representation.
    #[error(
        "failed to decode configuration JSON wire input ({category:?}) at line {line}, column {column}"
    )]
    Json {
        /// Broad Serde failure category.
        category: Category,
        /// One-based line reported by Serde, or zero when unavailable.
        line: usize,
        /// One-based column reported by Serde, or zero when unavailable.
        column: usize,
    },
    /// A future JSON adapter failure that has no dedicated configuration
    /// error variant yet.
    #[error("configuration JSON adapter failure: {0}")]
    Adapter(String),
    /// The decoded wire fields violate a configuration invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}
