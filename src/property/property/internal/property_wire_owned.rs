// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair

use qubit_value::ValueWireV1;
use qubit_value::ValueWireV1Seed;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeSeed;

fn deserialize_value_wire<'de, D>(deserializer: D) -> Result<ValueWireV1, D::Error>
where
    D: Deserializer<'de>,
{
    ValueWireV1Seed::new().deserialize(deserializer)
}

/// Owned wire representation used while deserializing a property.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::property) struct PropertyWireOwned {
    /// Property name.
    pub(in crate::property) name: String,
    /// Explicitly versioned property value.
    #[serde(deserialize_with = "deserialize_value_wire")]
    pub(in crate::property) value: ValueWireV1,
    /// Optional human-readable description.
    pub(in crate::property) description: Option<String>,
    /// Whether overriding is prohibited.
    pub(in crate::property) is_final: bool,
}
