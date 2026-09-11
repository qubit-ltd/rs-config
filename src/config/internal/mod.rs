// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Legacy and current Serde persistence representations.
mod config_serde_repr;
/// Accepted versioned and legacy wire representations.
mod config_wire;
/// Shared wire fields and version marker.
mod config_wire_fields;
mod config_wire_seed;
/// Owned version-one wire representation.
mod config_wire_v1;
/// Borrowed version-one wire representation.
mod config_wire_v1_ref;

pub(super) use config_serde_repr::ConfigSerdeRepr;
pub(super) use config_wire::ConfigWire;
pub(super) use config_wire_fields::ConfigWireFields;
pub(super) use config_wire_seed::AccountingConfigWireSeed;
pub(super) use config_wire_seed::JsonAdmittedConfigWireSeed;
pub(super) use config_wire_v1::ConfigWireV1;
pub(super) use config_wire_v1_ref::ConfigWireV1Ref;
