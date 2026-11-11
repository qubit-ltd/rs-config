// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Accounts for configuration wire decoding under configured resource limits.
mod accounting_config_wire_seed;
/// Legacy and current Serde persistence representations.
mod config_serde_repr;
/// Accepted versioned and legacy wire representations.
mod config_wire;
/// Shared wire fields and version marker.
mod config_wire_fields;
/// Owned version-one wire representation.
mod config_wire_v1;
/// Borrowed version-one wire representation.
mod config_wire_v1_ref;
/// Decodes configuration wire values after JSON-session admission.
mod json_admitted_config_wire_seed;
/// Safe diagnostics for bounded wire representation failures.
mod wire_diagnostics;
/// Assigns schema roles to decoded wire values.
mod wire_read_role;
/// Distinguishes an absent legacy version from an explicit version byte.
mod wire_version;

pub(super) use accounting_config_wire_seed::AccountingConfigWireSeed;
pub(super) use config_serde_repr::ConfigSerdeRepr;
pub(super) use config_wire::ConfigWire;
pub(super) use config_wire_fields::ConfigWireFields;
pub(super) use config_wire_v1::ConfigWireV1;
pub(super) use config_wire_v1_ref::ConfigWireV1Ref;
pub(super) use json_admitted_config_wire_seed::JsonAdmittedConfigWireSeed;
pub(super) use wire_diagnostics::decode_fields;
