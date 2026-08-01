// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair

use serde::Serialize;

use qubit_value::ValueWireRefV1;

/// Borrowed wire representation used while serializing a property.
#[derive(Serialize)]
pub(in crate::property) struct PropertyWireRef<'a> {
    /// Property name.
    pub(in crate::property) name: &'a str,
    /// Explicitly versioned property value.
    pub(in crate::property) value: ValueWireRefV1<'a>,
    /// Optional human-readable description.
    pub(in crate::property) description: &'a Option<String>,
    /// Whether overriding is prohibited.
    pub(in crate::property) is_final: bool,
}
