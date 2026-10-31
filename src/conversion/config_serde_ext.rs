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
/// Object member names are passed to Serde's key deserializer as strings;
/// string keys can be read as `String`, but integer or container key targets
/// are not parsed from those names.
/// Numeric keys can be parsed explicitly after deserializing string keys:
///
/// ```rust
/// use std::collections::HashMap;
/// use qubit_config::Config;
///
/// let mut config = Config::new();
/// config.set("ports.80", 8080_u16)?;
/// config.set("ports.443", 443_u16)?;
/// let named: HashMap<String, u16> = config.deserialize("ports")?;
/// let mut ports = HashMap::new();
/// for (name, value) in named {
///     let number = name.parse::<u16>()?;
///     if ports.insert(number, value).is_some() {
///         return Err(format!("duplicate numeric port key: {number}").into());
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Parsing errors propagate from `parse`; collecting normalized keys directly
/// into a map can overwrite earlier entries, so check duplicates when needed.
///
/// `UnknownFieldPolicy::Reject` only reports fields for which Serde calls
/// `deserialize_ignored_any` on this reader's source deserializer. Ordinary
/// derived structs report those fields, while flatten structs, untagged enums,
/// and custom `Deserialize` implementations that buffer fields can hide
/// discarded keys. Flatten maps may explicitly consume arbitrary keys.
///
/// Applications that require a closed schema can put
/// `#[serde(deny_unknown_fields)]` on ordinary struct targets, and on each
/// struct payload used by an untagged enum. For example:
///
/// ```rust
/// use qubit_config::Config;
/// #[derive(serde::Deserialize)]
/// #[serde(deny_unknown_fields)]
/// struct ClosedServer { port: u16 }
///
/// #[derive(serde::Deserialize)]
/// #[serde(untagged)]
/// enum ClosedChoice { Server(ClosedServer) }
///
/// let mut config = Config::new();
/// config.set("server.port", 8080_u16)?;
/// config.set("server.typo", true)?;
/// assert!(config.deserialize::<ClosedServer>("server").is_err());
/// assert!(config.deserialize::<ClosedChoice>("server").is_err());
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
///
/// Serde rejects unknown keys for these targets as a sanitized
/// deserialization error, not necessarily as `UnknownProperties` with a path
/// list. `deny_unknown_fields` cannot be combined with `flatten`; replace a
/// flatten struct with a closed nested struct, or validate its keys against
/// the application's schema before deserializing. `Ignore` changes only
/// unknown-field reporting and does not bypass input admission, conversion,
/// or resource limits.
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
    /// Deserializes without interpolation and rejects observable unknown
    /// fields.
    ///
    /// Unknown-field rejection records only calls to
    /// `deserialize_ignored_any` received by this reader's source deserializer
    /// and cannot see fields buffered by some flatten structs, untagged enums,
    /// or custom `Deserialize` implementations. Use
    /// `#[serde(deny_unknown_fields)]` for closed struct targets when the
    /// application must reject those keys; that produces a sanitized Serde
    /// deserialization error rather than a guaranteed `UnknownProperties`
    /// error with sorted paths.
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
    /// `UnknownFieldPolicy::Reject` reports only ignored-value callbacks
    /// received by this reader's source deserializer; it is not universal
    /// schema validation. The `Ignore` choice still performs complete input
    /// admission and honors this reader's conversion policies and resource
    /// limits.
    ///
    /// Conversion policy and resource limits come from this reader's
    /// ReadPolicy. Only actual String leaves interpolate; rich values keep
    /// their own types.
    ///
    /// Interpolation scope depends on the selected shape. When `prefix`
    /// selects dotted child keys, lookup starts in that selected subtree.
    /// When it selects an exact structured property such as a JSON object,
    /// lookup starts in this reader's scope; object member names are values,
    /// not configuration keys. Existing root and permitted environment
    /// fallback still applies.
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
