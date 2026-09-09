// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Structured reads through borrowed configuration sources.

use serde::de::DeserializeOwned;

use crate::ConfigDeserializeOptions;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::UnknownFieldPolicy;

/// Adds structured reads to Config and ConfigSection through ConfigReader.
///
/// Prefixes select an exact property or a subtree relative to the reader.
/// Empty prefixes select the entire visible scope. All selected input is
/// admitted before the target is visited, including ignored fields.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigDeserializeOptions, UnknownFieldPolicy};
/// #[derive(serde::Deserialize)]
/// struct Server { port: u16 }
/// let mut config = Config::new();
/// config.set("server.port", 8080_u16)?;
/// config.set("server.label", "public")?;
/// let server: Server = config.deserialize_with("server", ConfigDeserializeOptions {
///     unknown_fields: UnknownFieldPolicy::Ignore,
///     ..Default::default()
/// })?;
/// assert_eq!(server.port, 8080);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
pub trait ConfigSerdeExt: ConfigReader {
    /// Deserializes without interpolation and rejects unknown fields.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Relative property or subtree path; empty selects this
    ///   scope.
    ///
    /// # Errors
    ///
    /// Returns lookup, conversion, input-limit, conflict, unknown-field, or
    /// sanitized Serde errors with root-relative paths.
    fn deserialize<T: DeserializeOwned>(&self, prefix: impl AsRef<str>) -> ConfigResult<T> {
        self.deserialize_with(prefix, ConfigDeserializeOptions::default())
    }

    /// Deserializes with explicit interpolation and unknown-field behavior.
    ///
    /// Conversion policy and resource limits come from this reader's
    /// ReadPolicy. Only actual String leaves interpolate; rich values keep
    /// their own types.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Relative property or subtree path; empty selects this
    ///   scope.
    /// * `options` - Per-read interpolation and unknown-field choices.
    ///
    /// # Errors
    ///
    /// Returns lookup, conversion, interpolation, input-limit, conflict,
    /// unknown-field, or sanitized Serde errors retaining their source paths.
    fn deserialize_with<T: DeserializeOwned>(
        &self,
        prefix: impl AsRef<str>,
        options: ConfigDeserializeOptions,
    ) -> ConfigResult<T> {
        crate::structured_read::deserialize_from_reader(
            self,
            prefix.as_ref(),
            options.interpolate,
            matches!(options.unknown_fields, UnknownFieldPolicy::Reject),
        )
    }
}

impl<R: ConfigReader + ?Sized> ConfigSerdeExt for R {}
