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
pub(in crate::config) struct WireVersion(pub(in crate::config) Option<u8>);

impl<'de> Deserialize<'de> for WireVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u8::deserialize(deserializer).map(|version| Self(Some(version)))
    }
}
