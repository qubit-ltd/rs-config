// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Preserves whether a persisted wire version was explicitly supplied.

use serde::Deserialize;
use serde::Deserializer;

/// Distinguishes an absent legacy version from every explicitly supplied byte.
#[derive(Default)]
pub(in crate::config) struct WireVersion(
    /// The explicit version byte, or `None` when the legacy field was absent.
    pub(in crate::config) Option<u8>,
);

impl<'de> Deserialize<'de> for WireVersion {
    /// Deserializes an explicit version byte while leaving absence to
    /// `Default`.
    ///
    /// # Parameters
    /// - `deserializer`: Deserializer containing the supplied version byte.
    ///
    /// # Returns
    /// A wire version containing `Some(version)`.
    ///
    /// # Errors
    /// Returns the deserializer error when the supplied value is not a `u8`.
    ///
    /// # Type Parameters
    ///
    /// * `D` - Deserializer implementation containing the version value.
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u8::deserialize(deserializer).map(|version| Self(Some(version)))
    }
}
