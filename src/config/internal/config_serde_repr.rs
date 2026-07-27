// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::HashMap;

use serde::Deserialize;

use crate::Property;
use crate::options::ReadOptions;

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
    /// Runtime conversion and explicit interpolation options.
    #[serde(default)]
    pub(in crate::config) read_options: ReadOptions,
}
