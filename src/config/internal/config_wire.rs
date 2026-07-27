// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde::Deserialize;

use super::{
    ConfigSerdeRepr,
    ConfigWireV1,
};

/// Accepted persisted `Config` wire representations.
#[derive(Deserialize)]
#[serde(untagged)]
pub(in crate::config) enum ConfigWire {
    /// The explicit, stable V1 persistence format.
    V1(ConfigWireV1),
    /// The unversioned format emitted before the V1 persistence contract.
    Legacy(ConfigSerdeRepr),
}
