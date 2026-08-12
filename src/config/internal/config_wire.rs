// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeSeed;
use serde::de::Error as _;

use super::ConfigSerdeRepr;
use super::ConfigWireFields;
use super::ConfigWireSeed;
use super::ConfigWireV1;

/// Accepted persisted `Config` wire representations.
pub(in crate::config) enum ConfigWire {
    /// The explicit, stable V1 persistence format.
    V1(ConfigWireV1),
    /// The unversioned format emitted before the V1 persistence contract.
    Legacy(ConfigSerdeRepr),
}

impl<'de> Deserialize<'de> for ConfigWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigWireSeed::new(crate::ConfigWireLimits::default())
            .deserialize(deserializer)?
            .map_err(D::Error::custom)
    }
}

impl ConfigWire {
    /// Selects the versioned or legacy contract after common field decoding.
    pub(super) fn from_fields(
        fields: ConfigWireFields,
    ) -> Result<Self, String> {
        Ok(match fields.version.0 {
            Some(version) => {
                if fields.read_options.is_some() {
                    return Err(
                        "read_options is only accepted in legacy unversioned config wire"
                            .to_string(),
                    );
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
