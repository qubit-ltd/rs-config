// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Schema roles used to read native floating-point values from the Config wire.

/// Identifies wire positions that can request typed native float decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WireReadRole {
    /// An ordinary JSON location with no recognized wire-schema context.
    General,
    /// The configuration root; its `properties` member enters
    /// [`Self::Properties`].
    ConfigRoot,
    /// The property-name map; each member enters [`Self::Property`].
    Properties,
    /// A property wrapper; its `value` member enters [`Self::ValueEnvelope`].
    Property,
    /// A value wrapper; its `value` member enters [`Self::Payload`].
    ValueEnvelope,
    /// A payload whose `scalar` and `collection` members enter their variant
    /// roles.
    Payload,
    /// A scalar variant; `float32` and `float64` enter typed float roles.
    ScalarVariant,
    /// A collection variant; `float32` and `float64` enter typed sequence
    /// roles.
    CollectionVariant,
    /// A schema position requiring a native Float32 value, not a generic JSON
    /// number.
    Float32,
    /// A schema position requiring a native Float64 value, not a generic JSON
    /// number.
    Float64,
    /// A Float32 collection; each sequence element enters [`Self::Float32`].
    Float32Sequence,
    /// A Float64 collection; each sequence element enters [`Self::Float64`].
    Float64Sequence,
}

impl WireReadRole {
    /// Selects a child role from the current schema role and map key.
    ///
    /// Unrecognized keys and keys outside the wire schema fall back to
    /// [`Self::General`], so ordinary JSON numbers do not acquire typed-float
    /// semantics.
    pub(super) fn child_for_key(self, key: &str) -> Self {
        match (self, key) {
            (Self::ConfigRoot, "properties") => Self::Properties,
            (Self::Properties, _) => Self::Property,
            (Self::Property, "value") => Self::ValueEnvelope,
            (Self::ValueEnvelope, "value") => Self::Payload,
            (Self::Payload, "scalar") => Self::ScalarVariant,
            (Self::Payload, "collection") => Self::CollectionVariant,
            (Self::ScalarVariant, "float32") => Self::Float32,
            (Self::ScalarVariant, "float64") => Self::Float64,
            (Self::CollectionVariant, "float32") => Self::Float32Sequence,
            (Self::CollectionVariant, "float64") => Self::Float64Sequence,
            _ => Self::General,
        }
    }

    /// Selects the element role for a sequence payload.
    ///
    /// Only recognized Float32 and Float64 sequence roles retain a typed
    /// element role; every other sequence falls back to [`Self::General`].
    pub(super) fn sequence_element(self) -> Self {
        match self {
            Self::Float32Sequence => Self::Float32,
            Self::Float64Sequence => Self::Float64,
            _ => Self::General,
        }
    }
}
