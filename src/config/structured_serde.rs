// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Structured configuration deserialization façade.
// qubit-style: allow source-test-pair

use serde::de::DeserializeOwned;

use super::Config;
use crate::ConfigDeserializeOptions;
use crate::ConfigResult;
use crate::ConfigSerdeExt;

impl Config {
    /// Reads an exact property or subtree into an owned Serde target.
    ///
    /// An empty prefix selects the entire config. Dotted child keys become
    /// nested objects; an exact property coexisting with descendants conflicts.
    /// Typed scalar requests convert the original source, while deserialize_any
    /// uses natural JSON categories, including text for wide integers.
    ///
    /// This operation does not interpolate and rejects unknown fields. Use
    /// [`Self::deserialize_with`] for explicit alternatives. Rich Rust targets
    /// must request a supported Serde shape; universal round trips are not
    /// promised.
    ///
    /// # Errors
    ///
    /// Returns structured lookup, conversion, budget, conflict, unknown-field,
    /// or sanitized Serde errors with root-relative paths.
    pub fn deserialize<T: DeserializeOwned>(&self, prefix: impl AsRef<str>) -> ConfigResult<T> {
        ConfigSerdeExt::deserialize(self, prefix)
    }

    /// Reads an exact property or subtree with explicit per-read options.
    ///
    /// Interpolation visits actual String leaves once using scope, root, then
    /// permitted environment fallback. Ignored fields still undergo admission.
    /// Conversion policy and limits remain those of this config's ReadPolicy.
    ///
    /// # Errors
    ///
    /// Returns structured lookup, interpolation, conversion, budget, conflict,
    /// unknown-field, or sanitized Serde errors with root-relative paths.
    pub fn deserialize_with<T: DeserializeOwned>(
        &self,
        prefix: impl AsRef<str>,
        options: ConfigDeserializeOptions,
    ) -> ConfigResult<T> {
        ConfigSerdeExt::deserialize_with(self, prefix, options)
    }
}
