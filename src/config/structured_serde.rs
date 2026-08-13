// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Structured configuration deserialization façade.

use serde::de::DeserializeOwned;

use super::Config;
use crate::ConfigResult;
use crate::config_serde_ext::ConfigSerdeExt;

impl Config {
    // ========================================================================
    // Structured Config Deserialization (v0.4.0)
    // ========================================================================

    /// Deserializes the subtree at `prefix` through the configuration Serde
    /// view.
    ///
    /// String-backed scalar and collection projections apply this config's
    /// [`crate::options::ReadPolicy`] conversion policies without interpolating
    /// placeholders. Use [`Self::deserialize_interpolated`] for explicit
    /// interpolation.
    ///
    /// The Serde view is JSON-like: mappings, sequences, booleans, strings,
    /// numbers, and null values are exposed according to Serde's data model.
    /// This method does not promise the native conversion shapes used by
    /// [`Self::get`] for rich values such as durations or arbitrary-precision
    /// integers.
    ///
    /// When `prefix` is non-empty, an exact property named `prefix` is
    /// deserialized as the root value. If no exact property exists, child keys
    /// under `prefix` (prefix and trailing dot removed) form an object for
    /// `serde`, for example:
    ///
    /// ```rust
    /// #[derive(serde::Deserialize)]
    /// struct HttpOptions {
    ///     host: String,
    ///     port: u16,
    /// }
    /// ```
    ///
    /// can be populated from config keys `http.host` and `http.port` by calling
    /// `config.deserialize::<HttpOptions>("http")`. Defining both `http` and
    /// `http.*` is a [`crate::ConfigError::KeyConflict`], as are ambiguous
    /// dotted paths such as `a` and `a.b` inside the same deserialized
    /// object.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type, must implement `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix for the struct fields (`""` means the root map)
    ///
    /// # Returns
    ///
    /// The deserialized `T`.
    ///
    /// # Errors
    ///
    /// Returns the original [`crate::ConfigError`] when configuration lookup or
    /// conversion fails, preserving its kind, leaf path, and source. Unknown
    /// fields return [`crate::ConfigError::UnknownProperties`] with sorted
    /// root-relative paths. A mismatch raised only by `T`'s Serde
    /// implementation returns [`crate::ConfigError::DeserializeError`] at
    /// `prefix` with a fixed sanitized message. Use the explicit lenient
    /// methods when extra fields are part of the accepted contract.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Server {
    ///     host: String,
    ///     port: i32,
    /// }
    ///
    /// let mut config = Config::new();
    /// config.set("server.host", "localhost").unwrap();
    /// config.set("server.port", 8080).unwrap();
    ///
    /// let server: Server = config.deserialize("server").unwrap();
    /// assert_eq!(server.host, "localhost");
    /// assert_eq!(server.port, 8080);
    /// ```
    pub fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize(self, prefix)
    }

    /// Deserializes an exact value or subtree after interpolating strings.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type implementing `serde::de::DeserializeOwned`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Exact property key or dotted subtree path; an empty string
    ///   selects the root map.
    ///
    /// # Returns
    ///
    /// The interpolated and deserialized value.
    ///
    /// # Errors
    ///
    /// Returns lookup, interpolation, resource-limit, conversion, key-conflict,
    /// or sanitized Serde errors with their original configuration path.
    pub fn deserialize_interpolated<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize_interpolated(self, prefix)
    }

    /// Deserializes a subtree while explicitly ignoring fields not consumed by
    /// the target type.
    pub fn deserialize_lenient<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize_lenient(self, prefix)
    }

    /// Deserializes an interpolated subtree while explicitly ignoring fields
    /// not consumed by the target type.
    pub fn deserialize_interpolated_lenient<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        ConfigSerdeExt::deserialize_interpolated_lenient(self, prefix)
    }
}
