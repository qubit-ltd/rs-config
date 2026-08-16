// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Resource limits applied while ingesting configuration sources.
// qubit-style: allow multiple-public-types

use serde::Deserialize;
use serde::Serialize;

/// Default maximum source document size: 8 MiB.
pub const DEFAULT_MAX_SOURCE_INPUT_BYTES: usize = 8 * 1024 * 1024;
/// Default maximum number of properties emitted by one source.
pub const DEFAULT_MAX_SOURCE_PROPERTIES: usize = 65_536;
/// Default maximum number of parsed structural nodes.
pub const DEFAULT_MAX_SOURCE_NODES: usize = 262_144;
/// Default maximum number of child sources in one composite.
pub const DEFAULT_MAX_COMPOSITE_SOURCES: usize = 256;
/// Default maximum structured document nesting depth.
pub const DEFAULT_MAX_SOURCE_DEPTH: usize = 64;

/// Resource limits for one configuration source load.
///
/// Input bytes, emitted properties, parsed nodes, and admitted child sources
/// are cumulative budgets: every accepted consumption is retained for the
/// lifetime of one load session, while a rejected grouped consumption leaves
/// every local and aggregate scope unchanged. Nesting depth is a stateless
/// point limit checked independently for each observed path. These limits do
/// not synchronize access or manage resource lifetimes.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceLimits {
    max_input_bytes: usize,
    max_properties: usize,
    max_nodes: usize,
    max_sources: usize,
    max_nesting_depth: usize,
}

impl SourceLimits {
    /// Creates a builder initialized with the default source limits.
    #[inline]
    pub const fn builder() -> SourceLimitsBuilder {
        SourceLimitsBuilder::new()
    }

    /// Creates a source policy with every resource limit disabled.
    pub const fn unbounded() -> Self {
        Self {
            max_input_bytes: usize::MAX,
            max_properties: usize::MAX,
            max_nodes: usize::MAX,
            max_sources: usize::MAX,
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

    /// Returns the maximum number of parsed structural nodes.
    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }

    /// Returns the maximum number of child sources in a composite.
    pub const fn max_sources(self) -> usize {
        self.max_sources
    }

    /// Returns the maximum structured nesting depth.
    pub const fn max_nesting_depth(self) -> usize {
        self.max_nesting_depth
    }
}

/// Builder for [`SourceLimits`].
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimitsBuilder {
    max_input_bytes: usize,
    max_properties: usize,
    max_nodes: usize,
    max_sources: usize,
    max_nesting_depth: usize,
}

impl SourceLimitsBuilder {
    /// Creates a builder initialized with [`SourceLimits::default`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_SOURCE_INPUT_BYTES,
            max_properties: DEFAULT_MAX_SOURCE_PROPERTIES,
            max_nodes: DEFAULT_MAX_SOURCE_NODES,
            max_sources: DEFAULT_MAX_COMPOSITE_SOURCES,
            max_nesting_depth: DEFAULT_MAX_SOURCE_DEPTH,
        }
    }

    /// Sets the maximum accepted input byte length.
    #[inline]
    pub const fn max_input_bytes(mut self, maximum: usize) -> Self {
        self.max_input_bytes = maximum;
        self
    }

    /// Sets the maximum number of emitted properties.
    #[inline]
    pub const fn max_properties(mut self, maximum: usize) -> Self {
        self.max_properties = maximum;
        self
    }

    /// Sets the maximum number of parsed structural nodes.
    #[inline]
    pub const fn max_nodes(mut self, maximum: usize) -> Self {
        self.max_nodes = maximum;
        self
    }

    /// Sets the maximum number of child sources in a composite.
    #[inline]
    pub const fn max_sources(mut self, maximum: usize) -> Self {
        self.max_sources = maximum;
        self
    }

    /// Sets the maximum structured nesting depth.
    #[inline]
    pub const fn max_nesting_depth(mut self, maximum: usize) -> Self {
        self.max_nesting_depth = maximum;
        self
    }

    /// Builds the configured source limits.
    #[inline]
    pub const fn build(self) -> SourceLimits {
        SourceLimits {
            max_input_bytes: self.max_input_bytes,
            max_properties: self.max_properties,
            max_nodes: self.max_nodes,
            max_sources: self.max_sources,
            max_nesting_depth: self.max_nesting_depth,
        }
    }
}

impl Default for SourceLimitsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SourceLimits {
    fn default() -> Self {
        Self::builder().build()
    }
}
