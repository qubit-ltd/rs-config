// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Resource limits applied while ingesting configuration sources.

use serde::Deserialize;
use serde::Serialize;

/// Default maximum source document size: 8 MiB.
pub const DEFAULT_MAX_SOURCE_INPUT_BYTES: usize = 8 * 1024 * 1024;
/// Default maximum number of properties emitted by one source.
pub const DEFAULT_MAX_SOURCE_PROPERTIES: usize = 65_536;
/// Default maximum structured document nesting depth.
pub const DEFAULT_MAX_SOURCE_DEPTH: usize = 64;

/// Resource limits for one configuration source load.
///
/// Input bytes and emitted properties are cumulative budgets: every accepted
/// consumption is retained for the lifetime of one source load, while a
/// rejected consumption leaves the accumulated count unchanged. Nesting depth
/// is a stateless point limit checked independently for each observed path.
/// These limits do not synchronize access or manage resource lifetimes.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceLimits {
    max_input_bytes: usize,
    max_properties: usize,
    max_nesting_depth: usize,
}

impl SourceLimits {
    /// Creates a source policy with every resource limit disabled.
    pub const fn unbounded() -> Self {
        Self {
            max_input_bytes: usize::MAX,
            max_properties: usize::MAX,
            max_nesting_depth: usize::MAX,
        }
    }

    /// Returns the maximum accepted input byte length.
    pub const fn max_input_bytes(self) -> usize {
        self.max_input_bytes
    }

    /// Returns the maximum number of emitted properties.
    pub const fn max_properties(self) -> usize {
        self.max_properties
    }

    /// Returns the maximum structured nesting depth.
    pub const fn max_nesting_depth(self) -> usize {
        self.max_nesting_depth
    }

    /// Returns a copy with a bounded input byte length.
    pub const fn with_max_input_bytes(mut self, maximum: usize) -> Self {
        self.max_input_bytes = maximum;
        self
    }

    /// Returns a copy with a bounded property count.
    pub const fn with_max_properties(mut self, maximum: usize) -> Self {
        self.max_properties = maximum;
        self
    }

    /// Returns a copy with a bounded structured nesting depth.
    pub const fn with_max_nesting_depth(mut self, maximum: usize) -> Self {
        self.max_nesting_depth = maximum;
        self
    }
}

impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_SOURCE_INPUT_BYTES,
            max_properties: DEFAULT_MAX_SOURCE_PROPERTIES,
            max_nesting_depth: DEFAULT_MAX_SOURCE_DEPTH,
        }
    }
}
