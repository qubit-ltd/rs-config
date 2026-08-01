// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::HashMap;

use serde::{
    Deserialize,
    de::IgnoredAny,
};

use crate::Property;

/// Deserialization-only representation validated before constructing a config.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::config) struct ConfigSerdeRepr {
    /// Optional configuration description.
    #[serde(default)]
    pub(in crate::config) description: Option<String>,
    /// Properties indexed by their canonical names.
    #[serde(default)]
    pub(in crate::config) properties: HashMap<String, Property>,
    /// Legacy runtime options accepted for backward input compatibility and
    /// intentionally ignored.
    #[allow(dead_code)]
    #[serde(default)]
    pub(in crate::config) read_options: Option<IgnoredAny>,
}
