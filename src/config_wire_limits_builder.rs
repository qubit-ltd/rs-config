// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_budget::ResourceLimit;
use qubit_budget::json::JsonDecodeLimits;
use qubit_budget::json::JsonEncodeLimits;
use qubit_budget::json::JsonResource;

use super::ConfigWireLimitKind;
use super::ConfigWireLimits;

/// Builder for [`ConfigWireLimits`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigWireLimits;
///
/// let limits = ConfigWireLimits::builder().max_properties(10).build();
/// assert_eq!(limits.max_properties(), 10);
/// ```
#[must_use = "finish configuring and build these wire limits"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigWireLimitsBuilder {
    /// Limits for JSON decoding.
    pub(super) json_decode: JsonDecodeLimits<JsonResource, u64>,
    /// Limits for JSON encoding.
    pub(super) json_encode: JsonEncodeLimits<JsonResource, u64>,
    /// Maximum number of properties.
    pub(super) max_properties: u64,
    /// Maximum UTF-8 bytes in one property key.
    pub(super) max_property_key_bytes: u64,
}

impl ConfigWireLimitsBuilder {
    /// Creates a builder initialized with the default wire limits.
    ///
    /// # Returns
    ///
    /// A builder populated with the standard JSON and configuration limits.
    #[inline]
    pub fn new() -> Self {
        Self {
            json_decode: ConfigWireLimits::default_json_decode_limits(ConfigWireLimits::DEFAULT_MAX_INPUT_BYTES),
            json_encode: ConfigWireLimits::default_json_encode_limits(ConfigWireLimits::DEFAULT_MAX_OUTPUT_BYTES),
            max_properties: ConfigWireLimits::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes: ConfigWireLimits::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Sets the maximum complete JSON input length while preserving the other
    /// JSON decoding limits.
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum number of bytes admitted from JSON input.
    ///
    /// # Returns
    ///
    /// This builder with the input limit changed and other decoding limits
    /// preserved.
    #[inline]
    pub fn max_input_bytes(mut self, maximum: u64) -> Self {
        self.json_decode = self.json_decode.into_builder().max_input_bytes(maximum).build();
        self
    }

    /// Replaces the JSON decoding limits.
    ///
    /// # Parameters
    ///
    /// * `limits` - Complete JSON decoding profile to use.
    ///
    /// # Returns
    ///
    /// This builder with the decoding profile replaced.
    #[inline]
    pub fn json_decode(mut self, limits: JsonDecodeLimits<JsonResource, u64>) -> Self {
        self.json_decode = limits;
        self
    }

    /// Replaces the JSON encoding limits.
    ///
    /// # Parameters
    ///
    /// * `limits` - Complete JSON encoding profile to use.
    ///
    /// # Returns
    ///
    /// This builder with the encoding profile replaced.
    #[inline]
    pub fn json_encode(mut self, limits: JsonEncodeLimits<JsonResource, u64>) -> Self {
        self.json_encode = limits;
        self
    }

    /// Sets the maximum number of persisted properties.
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum property count accepted in a configuration.
    ///
    /// # Returns
    ///
    /// This builder with the property-count limit updated.
    #[inline]
    pub const fn max_properties(mut self, maximum: u64) -> Self {
        self.max_properties = maximum;
        self
    }

    /// Sets the maximum UTF-8 bytes in one property key.
    ///
    /// # Parameters
    ///
    /// * `maximum` - Maximum UTF-8 byte length for each property key.
    ///
    /// # Returns
    ///
    /// This builder with the property-key byte limit updated.
    #[inline]
    pub const fn max_property_key_bytes(mut self, maximum: u64) -> Self {
        self.max_property_key_bytes = maximum;
        self
    }

    /// Builds the configured wire limits.
    ///
    /// # Returns
    ///
    /// The wire limits assembled from the configured JSON profiles and property
    /// bounds.
    #[inline]
    pub const fn build(self) -> ConfigWireLimits {
        ConfigWireLimits {
            json_decode: self.json_decode,
            json_encode: self.json_encode,
            properties: ResourceLimit::new(ConfigWireLimitKind::Properties, self.max_properties),
            property_key_bytes: ResourceLimit::new(ConfigWireLimitKind::PropertyKeyBytes, self.max_property_key_bytes),
        }
    }
}

impl Default for ConfigWireLimitsBuilder {
    /// Creates a builder with the documented default limits.
    ///
    /// # Returns
    ///
    /// A builder initialized with the default JSON and property bounds.
    fn default() -> Self {
        Self::new()
    }
}
