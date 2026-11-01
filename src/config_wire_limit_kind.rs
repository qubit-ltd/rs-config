// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Resource categories specific to configuration wire envelopes.
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
/// let result = Config::decode_json_slice_with_limits(&bytes, limits);
/// assert!(matches!(
///     result,
///     Err(ConfigWireDecodeError::LimitExceeded {
///         kind: ConfigWireLimitKind::Properties,
///         ..
///     })
/// ));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigWireLimitKind {
    /// Number of persisted properties.
    Properties,
    /// UTF-8 bytes in one property key.
    PropertyKeyBytes,
}
