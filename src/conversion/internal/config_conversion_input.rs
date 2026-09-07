// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;
use serde_json::Value;

/// Owns exactly one conversion source and its associated session capability.
pub(in crate::config_value_deserializer) enum ConfigConversionInput<'session, 'policy, 'source> {
    /// A normal Serde value that has not been admitted as a scalar item.
    TopLevel {
        value: Value,
        session: &'session mut ConversionSession<'policy>,
    },
    /// A scalar value already charged to the bound conversion session.
    AdmittedScalar(AdmittedScalarItem<'session, 'policy, 'source>),
}
