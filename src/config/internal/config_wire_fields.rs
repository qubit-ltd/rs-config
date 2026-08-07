// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, de::IgnoredAny};

use crate::Property;

/// Common fields decoded before selecting the versioned or legacy contract.
///
/// Avoiding Serde's untagged-enum fallback here preserves detailed nested
/// deserialization errors, including the rejected property key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::config) struct ConfigWireFields {
    /// Optional stable format revision.
    #[serde(default)]
    pub(in crate::config) version: WireVersion,
    /// Optional human-readable configuration description.
    #[serde(default)]
    pub(in crate::config) description: Option<String>,
    /// Properties indexed by their persisted names.
    #[serde(default)]
    pub(in crate::config) properties: BTreeMap<String, Property>,
    /// Legacy runtime options accepted for backward input compatibility and
    /// intentionally ignored.
    #[serde(default)]
    pub(in crate::config) read_options: Option<IgnoredAny>,
}

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
