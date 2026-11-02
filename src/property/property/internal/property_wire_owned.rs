// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_value::ValueWireV1;
use qubit_value::ValueWireV1Seed;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeSeed;

/// Adapts Serde's field hook to the explicit V1 value-envelope seed.
///
/// The owned property representation uses this hook to decode its value
/// through `ValueWireV1Seed` and validate the V1 runtime container. The
/// surrounding decoder remains responsible for document-level resource
/// limits and raw-input admission; the seed does not replace those checks.
///
/// # Type Parameters
///
/// * `D` - Deserializer supplied for the property's value field.
///
/// # Parameters
///
/// * `deserializer` - Decoder positioned at one V1 value envelope.
///
/// # Returns
///
/// The decoded and validated V1 value envelope.
///
/// # Errors
///
/// Propagates the deserializer error when the envelope or runtime container
/// is invalid, including limits enforced by the surrounding decoder.
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
