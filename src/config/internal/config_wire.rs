// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde::{Deserialize, Deserializer, de::Error as _};

use super::config_wire_fields::ConfigWireFields;
use super::{ConfigSerdeRepr, ConfigWireV1};

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
