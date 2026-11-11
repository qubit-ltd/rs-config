// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::IgnoredAny;

use super::wire_version::WireVersion;
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
    /// intentionally ignored. `None` means the field was absent; `Some` means
    /// it was present and consumed, including when its JSON value was `null`.
    #[serde(default, deserialize_with = "deserialize_present_read_options")]
    pub(in crate::config) read_options: Option<IgnoredAny>,
}

/// Records that `read_options` appeared while consuming its value without
/// interpreting the legacy payload.
///
/// # Parameters
///
/// * `deserializer` - Deserializer positioned at the legacy field value.
///
/// # Returns
///
/// `Some(IgnoredAny)` when the field is present, including an explicit JSON
/// `null`, so callers can distinguish presence from the field's default.
///
/// # Errors
///
/// Returns the deserializer error if it cannot consume the field value.
///
/// # Type Parameters
///
/// * `D` - Deserializer implementation positioned at the legacy field value.
#[inline]
fn deserialize_present_read_options<'de, D>(deserializer: D) -> Result<Option<IgnoredAny>, D::Error>
where
    D: Deserializer<'de>,
{
    IgnoredAny::deserialize(deserializer).map(Some)
}
