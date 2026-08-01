// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Deserializer,
    de::{
        Error as _,
        IgnoredAny,
    },
};

use super::{
    ConfigSerdeRepr,
    ConfigWireV1,
};
use crate::Property;

/// Accepted persisted `Config` wire representations.
pub(in crate::config) enum ConfigWire {
    /// The explicit, stable V1 persistence format.
    V1(ConfigWireV1),
    /// The unversioned format emitted before the V1 persistence contract.
    Legacy(ConfigSerdeRepr),
}

/// Common fields decoded before selecting the versioned or legacy contract.
///
/// Avoiding Serde's untagged-enum fallback here preserves detailed nested
/// deserialization errors, including the rejected property key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigWireFields {
    /// Optional stable format revision.
    #[serde(default)]
    version: WireVersion,
    /// Optional human-readable configuration description.
    #[serde(default)]
    description: Option<String>,
    /// Properties indexed by their persisted names.
    #[serde(default)]
    properties: BTreeMap<String, Property>,
    /// Legacy runtime options accepted for backward input compatibility and
    /// intentionally ignored.
    #[serde(default)]
    read_options: Option<IgnoredAny>,
}

/// Distinguishes an absent legacy version from every explicitly supplied byte.
#[derive(Default)]
struct WireVersion(Option<u8>);

impl<'de> Deserialize<'de> for WireVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u8::deserialize(deserializer).map(|version| Self(Some(version)))
    }
}

impl<'de> Deserialize<'de> for ConfigWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ConfigWireFields::deserialize(deserializer)?;
        Ok(match fields.version.0 {
            Some(version) => {
                if fields.read_options.is_some() {
                    return Err(D::Error::custom(
                        "read_options is only accepted in legacy unversioned config wire",
                    ));
                }
                Self::V1(ConfigWireV1 {
                    version,
                    description: fields.description,
                    properties: fields.properties,
                })
            }
            None => Self::Legacy(ConfigSerdeRepr {
                description: fields.description,
                properties: fields.properties.into_iter().collect(),
                read_options: fields.read_options,
            }),
        })
    }
}
