// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use serde::Serialize;

use crate::config::Config;
use crate::options::ReadOptions;
use crate::Property;

/// Borrowed V1 persistence representation with deterministic property order.
#[derive(Serialize)]
pub(in crate::config) struct ConfigWireV1Ref<'a> {
    /// Stable format revision.
    version: u8,
    /// Optional human-readable configuration description.
    description: &'a Option<String>,
    /// Properties indexed by their canonical names in deterministic wire order.
    properties: BTreeMap<&'a str, &'a Property>,
    /// Runtime conversion and explicit interpolation options.
    read_options: &'a ReadOptions,
}

impl<'a> From<&'a Config> for ConfigWireV1Ref<'a> {
    /// Projects a runtime config into its stable V1 persistence format.
    #[inline]
    fn from(config: &'a Config) -> Self {
        Self {
            version: 1,
            description: &config.description,
            properties: config
                .properties
                .iter()
                .map(|(key, property)| (key.as_str(), property))
                .collect(),
            read_options: &config.read_options,
        }
    }
}
