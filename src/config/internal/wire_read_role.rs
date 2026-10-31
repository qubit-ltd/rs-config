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
    General,
    ConfigRoot,
    Properties,
    Property,
    ValueEnvelope,
    Payload,
    ScalarVariant,
    CollectionVariant,
    Float32,
    Float64,
    Float32Sequence,
    Float64Sequence,
}

impl WireReadRole {
    /// Selects a child role from the current schema role and map key.
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
    pub(super) fn sequence_element(self) -> Self {
        match self {
            Self::Float32Sequence => Self::Float32,
            Self::Float64Sequence => Self::Float64,
            _ => Self::General,
        }
    }
}
