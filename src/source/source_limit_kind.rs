// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configuration source resource dimensions.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;

/// Resource dimension enforced during source ingestion.
///
/// A limit violation is returned as `ConfigError::SourceLimitExceeded` and
/// carries the dimension that rejected the load.
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigError;
/// use qubit_config::source::{ConfigSource, PropertiesConfigSource, SourceLimitKind, SourceLimits};
///
/// let limits = SourceLimits::builder().max_properties(0).build();
/// let source = PropertiesConfigSource::builder()
///     .content("port=8080")
///     .limits(limits)
///     .build();
/// let error = source.load().unwrap_err();
/// assert!(matches!(error, ConfigError::SourceLimitExceeded {
///     kind: SourceLimitKind::PropertyCount, ..
/// }));
/// ```
#[non_exhaustive]
#[must_use = "use the returned value"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceLimitKind {
    /// Raw input byte count.
    InputBytes,
    /// Parsed or emitted assignment count.
    PropertyCount,
    /// Parsed structural node count.
    NodeCount,
    /// Cumulative descendant-source admissions within a scope; the scope
    /// root itself is not counted.
    SourceCount,
    /// Root-relative structured nesting depth.
    NestingDepth,
    /// Tokens admitted by a format parser before syntax-tree construction.
    ParserTokenCount,
    /// Parser structure depth for one input.
    ParserNestingDepth,
    /// Composite source execution nesting depth (fixed executor guard).
    SourceNestingDepth,
}

impl Display for SourceLimitKind {
    /// Formats this limit kind as a concise diagnostic label.
    /// # Parameters
    ///
    /// * `formatter` - Formatter receiving the human-readable limit label.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputBytes => formatter.write_str("input bytes"),
            Self::PropertyCount => formatter.write_str("property count"),
            Self::NodeCount => formatter.write_str("node count"),
            Self::SourceCount => formatter.write_str("source count"),
            Self::NestingDepth => formatter.write_str("nesting depth"),
            Self::ParserTokenCount => formatter.write_str("parser token count"),
            Self::ParserNestingDepth => formatter.write_str("parser nesting depth"),
            Self::SourceNestingDepth => formatter.write_str("source nesting depth"),
        }
    }
}
