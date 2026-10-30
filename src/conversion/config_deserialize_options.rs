// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Per-read choices for structured configuration deserialization.

use crate::UnknownFieldPolicy;

/// Controls interpolation and unknown-field reporting for one structured read.
///
/// By default, [`Self::interpolate`] is `false` and
/// [`Self::unknown_fields`] is [`UnknownFieldPolicy::Reject`]. Rejection covers
/// only unknown fields observable through Serde's ignored-field callback; it
/// is not complete schema validation for every target shape. See
/// [`UnknownFieldPolicy::Reject`] for the limitations involving flatten,
/// untagged enums, and custom deserializers, and for closed-schema
/// alternatives. Conversion policies and resource limits belong to the reader's
/// ReadPolicy.
///
/// # Examples
///
/// ```rust
/// use qubit_config::Config;
/// use qubit_config::ConfigDeserializeOptions;
/// use qubit_config::ConfigError;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Server {
///     port: u16,
/// }
///
/// let mut config = Config::new();
/// config.set("server.port", 8080_u16)?;
/// config.set("server.typo", true)?;
/// let result = config.deserialize_with::<Server>(
///     "server",
///     ConfigDeserializeOptions::default(),
/// );
/// assert!(matches!(result, Err(ConfigError::UnknownProperties { .. })));
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConfigDeserializeOptions {
    /// Whether to interpolate actual `String` leaves before target conversion.
    ///
    /// The default is `false`. Non-string rich values are not interpolated.
    pub interpolate: bool,
    /// How to handle fields exposed through Serde's ignored-field callback.
    ///
    /// The default is [`UnknownFieldPolicy::Reject`]. It rejects observable
    /// unknown fields, not every field discarded by every target shape. See
    /// [`UnknownFieldPolicy::Reject`] for flatten, untagged-enum, and custom
    /// deserializer limitations and the closed-schema alternatives.
    pub unknown_fields: UnknownFieldPolicy,
}
