// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Configuration-specific profiles layered on rs-budget JSON limits.

use qubit_budget::ResourceLimit;
use qubit_budget::json::JsonDecodeLimits;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueLimits;

// Defines the wire limit category used by public limits and errors.
#[path = "config_wire_limit_kind.rs"]
mod config_wire_limit_kind;
pub use config_wire_limit_kind::ConfigWireLimitKind;
// Defines the bounded wire decoding error.
#[path = "config_wire_decode_error.rs"]
mod config_wire_decode_error;
pub use config_wire_decode_error::ConfigWireDecodeError;
// Defines limit construction without exposing stored limit fields.
#[path = "config_wire_limits_builder.rs"]
mod config_wire_limits_builder;
pub use config_wire_limits_builder::ConfigWireLimitsBuilder;

/// Limits applied while decoding or encoding a complete configuration wire
/// document.
///
/// [`Default`] builds the standard bounded wire profile. [`Self::from_json`]
/// retains the caller-provided JSON decoding and encoding profiles and applies
/// the default configuration property limits. Use
/// [`Config::decode_json_slice`](crate::Config::decode_json_slice) for
/// raw-input admission: ordinary Serde `Deserialize` does not expose the
/// original input bytes needed for that check.
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
#[must_use = "use the returned value"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigWireLimits {
    /// Limits for JSON decoding.
    json_decode: JsonDecodeLimits<JsonResource, u64>,
    /// Limits for JSON encoding.
    json_encode: JsonEncodeLimits<JsonResource, u64>,
    /// Configuration property-count limit.
    properties: ResourceLimit<ConfigWireLimitKind, u64>,
    /// Per-property key byte limit.
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
            properties: ResourceLimit::new(ConfigWireLimitKind::Properties, Self::DEFAULT_MAX_PROPERTIES),
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
    #[must_use = "use the returned value"]
    #[inline(always)]
    pub const fn max_properties(self) -> u64 {
        self.properties.maximum()
    }

    /// Returns the maximum UTF-8 bytes in one property key.
    #[must_use = "use the returned value"]
    #[inline(always)]
    pub const fn max_property_key_bytes(self) -> u64 {
        self.property_key_bytes.maximum()
    }

    /// Returns the complete property-count point limit.
    pub(crate) const fn properties_limit(&self) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.properties
    }

    /// Returns the complete property-key byte point limit.
    pub(crate) const fn property_key_bytes_limit(&self) -> &ResourceLimit<ConfigWireLimitKind, u64> {
        &self.property_key_bytes
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
    fn default_json_decode_limits(max_input_bytes: u64) -> JsonDecodeLimits<JsonResource, u64> {
        JsonDecodeLimits::builder()
            .max_input_bytes(max_input_bytes)
            .value_limits(Self::default_json_value_limits())
            .build()
    }

    /// Builds the JSON encoding profile used by configuration wire output.
    fn default_json_encode_limits(max_output_bytes: u64) -> JsonEncodeLimits<JsonResource, u64> {
        JsonEncodeLimits::builder()
            .max_output_bytes(max_output_bytes)
            .value_limits(Self::default_json_value_limits())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigWireLimits;

    #[test]
    fn private_limit_accessors_are_exercised() {
        let original = ConfigWireLimits::builder()
            .max_properties(17)
            .max_property_key_bytes(23)
            .build();
        let rebuilt = ConfigWireLimits::builder_from(&original).build();
        assert_eq!(rebuilt.properties_limit().maximum(), 17);
        assert_eq!(rebuilt.property_key_bytes_limit().maximum(), 23);

        let json_limits = ConfigWireLimits::from_json(
            super::JsonDecodeLimits::builder().max_input_bytes(29).build(),
            super::JsonEncodeLimits::builder().max_output_bytes(31).build(),
        );
        assert_eq!(json_limits.json_decode().max_input_bytes(), Some(29));
        assert_eq!(json_limits.json_encode().max_output_bytes(), Some(31));
        assert_eq!(
            json_limits.properties_limit().maximum(),
            ConfigWireLimits::DEFAULT_MAX_PROPERTIES
        );
        assert_eq!(
            json_limits.property_key_bytes_limit().maximum(),
            ConfigWireLimits::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        );
    }
}
