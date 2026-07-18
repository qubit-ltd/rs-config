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
use crate::constants::DEFAULT_MAX_SUBSTITUTION_DEPTH;
use crate::options::ConfigReadOptions;

/// Deserialization-only representation validated before constructing a config.
#[derive(Deserialize)]
pub(in crate::config) struct ConfigSerdeRepr {
    /// Optional configuration description.
    #[serde(default)]
    pub(in crate::config) description: Option<String>,
    /// Properties indexed by their canonical names.
    #[serde(default)]
    pub(in crate::config) properties: HashMap<String, Property>,
    /// Whether variable substitution is enabled.
    #[serde(default = "default_variable_substitution")]
    pub(in crate::config) enable_variable_substitution: bool,
    /// Maximum recursive variable-substitution depth.
    #[serde(default = "default_substitution_depth")]
    pub(in crate::config) max_substitution_depth: usize,
    /// Runtime conversion and substitution options.
    #[serde(default)]
    pub(in crate::config) read_options: ConfigReadOptions,
}

/// Returns the default variable-substitution setting.
///
/// # Returns
///
/// `true`, matching [`crate::Config::new`].
#[inline(always)]
const fn default_variable_substitution() -> bool {
    true
}

/// Returns the default maximum substitution depth.
///
/// # Returns
///
/// [`DEFAULT_MAX_SUBSTITUTION_DEPTH`].
#[inline(always)]
const fn default_substitution_depth() -> usize {
    DEFAULT_MAX_SUBSTITUTION_DEPTH
}
