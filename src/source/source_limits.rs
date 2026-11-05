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
/// Default maximum number of parsed structural nodes.
pub const DEFAULT_MAX_SOURCE_NODES: usize = 262_144;
/// Default maximum number of child sources in one composite.
pub const DEFAULT_MAX_COMPOSITE_SOURCES: usize = 256;
/// Default maximum structured document nesting depth.
pub const DEFAULT_MAX_SOURCE_DEPTH: usize = 64;
/// Default maximum number of parser tokens accepted by one source and its
/// ancestors.
pub const DEFAULT_MAX_SOURCE_PARSER_TOKENS: usize = 1_048_576;
/// Default maximum parser nesting depth for one input.
pub const DEFAULT_MAX_SOURCE_PARSER_DEPTH: usize = 128;

// Defines limit construction without exposing stored limit fields.
#[path = "source_limits_builder.rs"]
mod source_limits_builder;
pub use source_limits_builder::SourceLimitsBuilder;

/// Resource limits for one configuration source load.
///
/// Input bytes, emitted properties, parsed nodes, parser tokens, and admitted
/// child sources are cumulative budgets: every accepted consumption is
/// retained for the lifetime of one load session, while a rejected grouped
/// consumption leaves every local and aggregate scope unchanged. Parser tokens
/// count parser work before syntax-tree construction and are distinct from
/// semantic nodes counted while flattening. Built-in TOML sources charge every
/// lexer token, including whitespace, comments, newlines, and EOF; YAML sources
/// charge every scanner token. TOML parser depth is a conservative structural
/// bound, while YAML parser depth measures nested collections. Neither is
/// equivalent to semantic nesting depth. Semantic depth and parser nesting
/// depth are stateless point limits checked independently for each input.
/// These limits do not synchronize access or manage resource lifetimes.
/// The default parser-token budget is 1,048,576 tokens per source ancestry,
/// and the default parser-depth limit is 128 for each input.
///
/// Enforcement stages depend on the source adapter. TOML and YAML admit input
/// bytes, parser tokens, and parser nesting depth before constructing a syntax
/// tree, then check node, assignment, and semantic depth limits while
/// flattening it. Parser limits bound admitted work but do not guarantee a
/// strict allocation or CPU ceiling.
///
/// See the [source-limit guide](https://github.com/qubit-ltd/rs-config/blob/main/doc/user_guide.md#configure-source-limits).
///
/// Limits can be customized per source or for an entire composite. A zero
/// threshold rejects any positive consumption of that resource.
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigError;
/// use qubit_config::source::{ConfigSource, PropertiesConfigSource, SourceLimitKind, SourceLimits};
///
/// let limits = SourceLimits::builder()
///     .max_input_bytes(1024)
///     .max_properties(0)
///     .build();
/// assert_eq!(limits.max_input_bytes(), 1024);
/// assert_eq!(limits.max_properties(), 0);
///
/// let source = PropertiesConfigSource::builder()
///     .content("port=8080")
///     .limits(limits)
///     .build();
/// assert!(matches!(source.load(), Err(ConfigError::SourceLimitExceeded {
///     kind: SourceLimitKind::PropertyCount, ..
/// })));
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceLimits {
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
            max_parser_tokens: usize::MAX,
            max_parser_nesting_depth: usize::MAX,
        }
    }

    /// Returns the maximum accepted input byte length.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_input_bytes(self) -> usize {
        self.max_input_bytes
    }

    /// Returns the maximum number of emitted properties.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_properties(self) -> usize {
        self.max_properties
    }

    /// Returns the maximum number of parsed structural nodes.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }

    /// Returns the maximum number of child sources in a composite.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_sources(self) -> usize {
        self.max_sources
    }

    /// Returns the maximum structured nesting depth.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_nesting_depth(self) -> usize {
        self.max_nesting_depth
    }

    /// Returns the maximum parser tokens accepted by this source and its
    /// ancestors.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_parser_tokens(self) -> usize {
        self.max_parser_tokens
    }

    /// Returns the maximum parser nesting depth for one input, with root depth
    /// zero.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn max_parser_nesting_depth(self) -> usize {
        self.max_parser_nesting_depth
    }
}

impl Default for SourceLimits {
    /// Creates the default value for this type.
    #[inline]
    fn default() -> Self {
        Self::builder().build()
    }
}
