// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::DEFAULT_MAX_COMPOSITE_SOURCES;
use super::DEFAULT_MAX_SOURCE_DEPTH;
use super::DEFAULT_MAX_SOURCE_INPUT_BYTES;
use super::DEFAULT_MAX_SOURCE_NODES;
use super::DEFAULT_MAX_SOURCE_PARSER_DEPTH;
use super::DEFAULT_MAX_SOURCE_PARSER_TOKENS;
use super::DEFAULT_MAX_SOURCE_PROPERTIES;
use super::SourceLimits;

/// Builder for [`SourceLimits`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::ConfigSource;
/// use qubit_config::source::PropertiesConfigSource;
/// use qubit_config::source::SourceLimits;
///
/// let limits = SourceLimits::builder().max_properties(1).build();
/// let source = PropertiesConfigSource::builder()
///     .content("server.port=8080")
///     .limits(limits)
///     .build();
/// let config = source.load().unwrap();
/// assert_eq!(config.get::<u16>("server.port").unwrap(), 8080);
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimitsBuilder {
    /// Maximum input bytes accepted from one source.
    max_input_bytes: usize,
    /// Maximum properties emitted by one source.
    max_properties: usize,
    /// Maximum structural nodes parsed by one source.
    max_nodes: usize,
    /// Maximum child sources in one composite.
    max_sources: usize,
    /// Maximum nesting depth of structured input.
    max_nesting_depth: usize,
    /// Maximum parser tokens accepted before syntax-tree construction.
    max_parser_tokens: usize,
    /// Maximum parser nesting depth for one input.
    max_parser_nesting_depth: usize,
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
            max_parser_tokens: DEFAULT_MAX_SOURCE_PARSER_TOKENS,
            max_parser_nesting_depth: DEFAULT_MAX_SOURCE_PARSER_DEPTH,
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

    /// Sets the maximum parser tokens accepted by this source and its
    /// ancestors.
    #[inline]
    pub const fn max_parser_tokens(mut self, maximum: usize) -> Self {
        self.max_parser_tokens = maximum;
        self
    }

    /// Sets the maximum parser nesting depth for one input, with root depth
    /// zero.
    #[inline]
    pub const fn max_parser_nesting_depth(mut self, maximum: usize) -> Self {
        self.max_parser_nesting_depth = maximum;
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
            max_parser_tokens: self.max_parser_tokens,
            max_parser_nesting_depth: self.max_parser_nesting_depth,
        }
    }
}

impl Default for SourceLimitsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
