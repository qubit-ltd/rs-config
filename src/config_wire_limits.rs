// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
// =============================================================================

//! Resource limits and errors for bounded configuration wire decoding.

use qubit_value::{ValueWireDecodeError, WireLimits};
use thiserror::Error;

/// Resource categories specific to configuration wire envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigWireLimitKind {
    /// Number of persisted properties.
    Properties,
    /// UTF-8 bytes in one property key.
    PropertyKeyBytes,
}

/// Limits applied while decoding a complete configuration wire document.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigWireLimits {
    wire: WireLimits,
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
            wire: WireLimits::new(max_input_bytes),
            max_properties: Self::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes: Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Creates configuration limits from an already configured shared budget.
    #[inline(always)]
    pub const fn from_wire(wire: WireLimits) -> Self {
        Self {
            wire,
            max_properties: Self::DEFAULT_MAX_PROPERTIES,
            max_property_key_bytes: Self::DEFAULT_MAX_PROPERTY_KEY_BYTES,
        }
    }

    /// Sets the shared value and JSON wire limits.
    #[inline(always)]
    #[must_use = "the configured wire limits should be used"]
    pub const fn with_wire(mut self, wire: WireLimits) -> Self {
        self.wire = wire;
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
    pub const fn with_max_property_key_bytes(mut self, max_property_key_bytes: usize) -> Self {
        self.max_property_key_bytes = max_property_key_bytes;
        self
    }

    /// Returns the shared value and JSON wire limits.
    #[inline(always)]
    pub const fn wire(self) -> WireLimits {
        self.wire
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
        Self::new(WireLimits::DEFAULT_MAX_INPUT_BYTES)
    }
}

/// Error returned by bounded configuration wire decoding.
#[derive(Debug, Error)]
pub enum ConfigWireDecodeError {
    /// A shared Value/JSON resource limit was exceeded.
    #[error(transparent)]
    Value(#[from] ValueWireDecodeError),
    /// A configuration-specific resource limit was exceeded.
    #[error("configuration wire {kind:?} value {value} exceeds the limit of {maximum}")]
    LimitExceeded {
        /// Resource category that exceeded its limit.
        kind: ConfigWireLimitKind,
        /// Observed resource value.
        value: usize,
        /// Largest permitted resource value.
        maximum: usize,
    },
    /// The JSON document was syntactically valid enough to parse but did not
    /// satisfy the configuration wire contract.
    #[error("failed to decode configuration JSON wire input: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// The decoded wire fields violate a configuration invariant.
    #[error("invalid configuration wire value: {0}")]
    InvalidConfig(String),
}
