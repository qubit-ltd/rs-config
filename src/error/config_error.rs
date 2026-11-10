// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Configuration Error Type
//!
//! Defines all possible error scenarios in the configuration system.

use std::borrow::Cow;

use qubit_budget::BudgetError;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_value::ValueError;
use qubit_value::ValueMissing;
use thiserror::Error;

use crate::ConfigErrorKind;
use crate::ConfigPathViolation;
use crate::SourceLimitKind;
use crate::SourceLocation;

/// Escapes control characters in text included in a diagnostic display.
///
/// Text without control characters is borrowed unchanged. Other characters,
/// including visible Unicode, quotes, and literal backslashes, are preserved.
///
/// # Parameters
///
/// * `text` - Diagnostic text to escape when it contains control characters.
fn escape_diagnostic(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() {
            escaped.extend(ch.escape_debug());
        } else {
            escaped.push(ch);
        }
    }
    Cow::Owned(escaped)
}

/// Joins diagnostic text after escaping control characters in each item.
///
/// # Parameters
///
/// * `items` - Diagnostic items to escape and join in their original order.
/// * `separator` - Text inserted between the escaped items.
fn join_diagnostics(items: &[String], separator: &str) -> String {
    items
        .iter()
        .map(|item| escape_diagnostic(item))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Configuration error type.
///
/// Inspect [`Self::kind`], [`Self::path`], and [`Self::source_index`] for
/// structured diagnostics instead of parsing the display message.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigError, ConfigErrorKind};
///
/// let config = Config::new();
/// let missing = config.get::<i32>("server.port").unwrap_err();
/// assert_eq!(missing.kind(), ConfigErrorKind::PropertyNotFound);
/// assert_eq!(missing.path(), Some("server.port"));
/// assert_eq!(missing.source_index(), None);
///
/// let mut config = Config::new();
/// config.set("ports", vec!["80", "invalid"])?;
/// let conversion = config.get::<Vec<u16>>("ports").unwrap_err();
/// assert_eq!(conversion.kind(), ConfigErrorKind::Conversion);
/// assert_eq!(conversion.path(), Some("ports"));
/// assert_eq!(conversion.source_index(), Some(1));
/// # Ok::<(), ConfigError>(())
/// ```
#[non_exhaustive]
#[must_use]
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Configuration property key is not canonical.
    #[error("Invalid configuration key '{key}': {violation}", key = escape_diagnostic(key))]
    InvalidKey {
        /// Rejected property key.
        key: String,
        /// Structural key violation.
        violation: ConfigPathViolation,
    },

    /// A configuration source supplied a non-canonical property key.
    #[error("Invalid configuration key '{key}' in source '{source_id}': {violation}", key = escape_diagnostic(key), source_id = escape_diagnostic(source_id))]
    SourceInvalidKey {
        /// Stable source identifier or path.
        source_id: String,
        /// Rejected property key.
        key: String,
        /// Structural key violation.
        violation: ConfigPathViolation,
    },

    /// Configuration section path is not canonical.
    #[error("Invalid configuration path '{path}': {violation}", path = escape_diagnostic(path))]
    InvalidPath {
        /// Rejected section path.
        path: String,
        /// Structural path violation.
        violation: ConfigPathViolation,
    },

    /// A configuration source exceeded a resource limit.
    #[error("Configuration source '{source_id}' exceeded budget '{budget_id}': {source}", source_id = escape_diagnostic(source_id), budget_id = escape_diagnostic(budget_id), source = escape_diagnostic(&source.to_string()))]
    SourceLimitExceeded {
        /// Source identifier or path.
        source_id: String,
        /// Identifier of the local or aggregate budget that rejected usage.
        budget_id: String,
        /// Bounded resource dimension.
        kind: SourceLimitKind,
        /// Configured maximum.
        limit: usize,
        /// Minimum resource usage observed before rejection.
        observed_at_least: usize,
        /// Structured budget failure.
        #[source]
        source: BudgetError<SourceLimitKind, usize>,
    },

    /// Property not found.
    #[error("Property not found: {}", escape_diagnostic(.0))]
    PropertyNotFound(
        /// Missing configuration path.
        String,
    ),

    /// None of several candidate properties was found.
    #[error(
        "None of the candidate properties was found: {}",
        join_diagnostics(.paths, ", ")
    )]
    PropertyCandidatesNotFound {
        /// Missing root-relative configuration paths in lookup order.
        paths: Vec<String>,
    },

    /// Property has no value.
    #[error("Property '{}' has no value", escape_diagnostic(.0))]
    PropertyHasNoValue(
        /// Configuration path whose property has no usable value.
        String,
    ),

    /// Type mismatch at a specific key/path.
    #[error("Type mismatch at '{key}': expected {expected}, actual {actual}", key = escape_diagnostic(key))]
    TypeMismatch {
        /// Configuration key/path where the mismatch occurred.
        key: String,
        /// Expected type.
        expected: DataType,
        /// Actual type.
        actual: DataType,
    },

    /// Structured, value-redacted conversion failure.
    #[error("Type conversion failed at '{key}': {source}", key = escape_diagnostic(key), source = escape_diagnostic(&source.to_string()))]
    ConversionError {
        /// Configuration key/path where the conversion failed.
        key: String,
        /// Zero-based index in the original source collection, when relevant.
        source_index: Option<usize>,
        /// Value-free conversion failure details.
        #[source]
        source: DataConversionError,
    },

    /// A non-negative integer exceeds the range supported by this platform's
    /// `usize`.
    #[error("configuration value at '{key}' exceeds the platform usize range", key = escape_diagnostic(key))]
    PlatformIntegerOutOfRange {
        /// Configuration key/path where the value was read.
        key: String,
        /// Zero-based index of the selected source collection item, when
        /// applicable.
        source_index: Option<usize>,
    },

    /// Value-layer error not covered by a specialized configuration mapping.
    #[error("Value error at '{key}': {source}", key = escape_diagnostic(key), source = escape_diagnostic(&source.to_string()))]
    ValueError {
        /// Configuration key/path where the value access failed.
        key: String,
        /// Original value-layer error.
        #[source]
        source: ValueError,
    },

    /// Variable substitution failed.
    #[error("Variable substitution failed at '{path}': {message}", path = escape_diagnostic(path), message = escape_diagnostic(message))]
    SubstitutionError {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Human-readable resolution failure.
        message: String,
    },

    /// A substitution error tied to an element of a configuration collection.
    #[error("{source}")]
    IndexedSubstitutionError {
        /// Zero-based index in the original source collection.
        source_index: usize,
        /// Original substitution error, whose category and path are preserved.
        #[source]
        source: Box<ConfigError>,
    },

    /// Variable substitution depth exceeded.
    #[error("Variable substitution at '{path}' exceeded maximum depth: {max_depth}", path = escape_diagnostic(path))]
    SubstitutionDepthExceeded {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Maximum permitted recursive expansion depth.
        max_depth: usize,
    },

    /// Variable substitution resolved too many placeholders.
    #[error("Variable substitution at '{path}' exceeded maximum expansions: {max_expansions}", path = escape_diagnostic(path))]
    SubstitutionExpansionLimitExceeded {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Maximum permitted placeholder-resolution count.
        max_expansions: usize,
    },

    /// Variable substitution produced an oversized value.
    #[error("Variable substitution at '{path}' exceeded maximum output bytes: {max_output_bytes}", path = escape_diagnostic(path))]
    SubstitutionOutputTooLarge {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Maximum permitted UTF-8 byte length.
        max_output_bytes: usize,
    },

    /// Variable substitution cycle detected.
    #[error(
        "Variable substitution cycle at '{path}': {}",
        join_diagnostics(chain, " -> "),
        path = escape_diagnostic(path)
    )]
    SubstitutionCycle {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Variable chain that forms the cycle.
        chain: Vec<String>,
    },

    /// Configuration merge failed.
    #[error("Configuration merge failed: {}", escape_diagnostic(.0))]
    MergeError(
        /// Human-readable merge failure.
        String,
    ),

    /// Property is final and cannot be overridden.
    #[error("Property '{}' is final and cannot be overridden", escape_diagnostic(.0))]
    PropertyIsFinal(
        /// Configuration path protected against mutation.
        String,
    ),

    /// A source property could not be merged because the target property is
    /// final.
    #[error("Property '{key}' is final and cannot be overridden by source '{source_id}'", key = escape_diagnostic(key), source_id = escape_diagnostic(source_id))]
    SourcePropertyIsFinal {
        /// Stable identifier of the incoming source at the merge boundary.
        source_id: String,
        /// Configuration path protected against mutation.
        key: String,
    },

    /// Configuration key path cannot be represented without ambiguity.
    #[error(
        "Configuration key conflict at '{path}': existing {existing}, incoming {incoming}",
        path = .path.escape_debug(),
        existing = .existing.escape_debug(),
        incoming = .incoming.escape_debug(),
    )]
    KeyConflict {
        /// Source identifier when the conflict came from a source loader.
        source_id: Option<String>,
        /// Conflicting configuration key/path.
        path: String,
        /// Existing value or path shape.
        existing: String,
        /// Incoming value or path shape.
        incoming: String,
    },

    /// Structured deserialization found fields not consumed by the target.
    #[error("Unknown configuration properties: {}", join_diagnostics(paths, ", "))]
    UnknownProperties {
        /// Root-relative configuration paths not recognized by the target
        /// type.
        paths: Vec<String>,
    },

    /// I/O error not associated with a named configuration source.
    #[error("IO error: {}", escape_diagnostic(&.0.to_string()))]
    IoError(
        /// Underlying I/O failure.
        #[from]
        std::io::Error,
    ),

    /// I/O error raised while loading a named configuration source.
    #[error("IO error in source '{source_id}': {source}", source_id = escape_diagnostic(source_id), source = escape_diagnostic(&source.to_string()))]
    SourceIoError {
        /// Stable source identifier or path.
        source_id: String,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// Parse error not associated with a named configuration source.
    #[error("Parse error: {}", escape_diagnostic(.0))]
    ParseError(
        /// Human-readable source parsing failure.
        String,
    ),

    /// Parse error raised while loading a named configuration source.
    #[error("Parse error in source '{source_id}': {message}", source_id = escape_diagnostic(source_id), message = escape_diagnostic(message))]
    SourceParseError {
        /// Stable source identifier or path.
        source_id: String,
        /// Optional configuration path within the source.
        path: Option<String>,
        /// Optional zero-based source collection index.
        source_index: Option<usize>,
        /// Human-readable source parsing failure.
        message: String,
        /// Structured physical source coordinates, when known.
        location: Option<SourceLocation>,
    },

    /// Deserialization error for structured config mapping.
    #[error("Deserialization error at '{path}': {message}", path = escape_diagnostic(path), message = escape_diagnostic(message))]
    DeserializeError {
        /// Config prefix/path being deserialized.
        path: String,
        /// Error message.
        message: String,
        /// Original structured error when it came from config parsing.
        #[source]
        source: Option<Box<ConfigError>>,
    },

    /// Other error.
    #[error("Configuration error: {}", escape_diagnostic(.0))]
    Other(
        /// Human-readable uncategorized failure.
        String,
    ),
}

impl ConfigError {
    /// Maps a common data conversion error to a keyed configuration error.
    ///
    /// # Parameters
    ///
    /// * `key` - Configuration path associated with the conversion.
    /// * `error` - Structured conversion failure to classify.
    ///
    /// # Returns
    ///
    /// A missing-value or conversion error retaining `key`.
    #[inline]
    pub fn from_data_conversion_error(key: &str, error: DataConversionError) -> Self {
        if error.is_missing() {
            Self::ValueError {
                key: key.to_string(),
                source: ValueError::from(error),
            }
        } else {
            Self::ConversionError {
                key: key.to_string(),
                source_index: None,
                source: error,
            }
        }
    }

    /// Creates an I/O error associated with a named configuration source.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable identifier or path for the configuration source.
    /// - `source`: underlying operating-system I/O failure.
    ///
    /// # Returns
    ///
    /// Returns a source-aware I/O configuration error.
    pub(crate) fn source_io_error(source_id: impl Into<String>, source: std::io::Error) -> Self {
        Self::SourceIoError {
            source_id: source_id.into(),
            source,
        }
    }

    /// Creates a parse error associated with a named configuration source.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable identifier or path for the configuration source.
    /// - `message`: human-readable parse failure description.
    ///
    /// # Returns
    ///
    /// Returns a source-aware parse error without path or collection-index
    /// context.
    pub(crate) fn source_parse_error(source_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SourceParseError {
            source_id: source_id.into(),
            path: None,
            source_index: None,
            message: message.into(),
            location: None,
        }
    }

    /// Creates a source-aware parse error with optional path and index context.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable identifier or path for the configuration source.
    /// - `path`: optional source-relative configuration path.
    /// - `source_index`: optional original collection index.
    /// - `message`: human-readable parse failure description.
    ///
    /// # Returns
    ///
    /// Returns a parse error retaining the supplied source, path, and index
    /// context while including present coordinates in its message.
    pub(crate) fn source_parse_error_at(
        source_id: impl Into<String>,
        path: Option<String>,
        source_index: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        let mut message = message.into();
        if let Some(path) = path.as_deref() {
            message.push_str(&format!(" at path '{path}'"));
        }
        if let Some(index) = source_index {
            message.push_str(&format!(" at source index {index}"));
        }
        Self::SourceParseError {
            source_id: source_id.into(),
            path,
            source_index,
            message,
            location: None,
        }
    }

    /// Maps a value-layer error while retaining its list source index.
    ///
    /// # Parameters
    ///
    /// * `key` - Configuration path associated with the value operation.
    /// * `error` - Value-layer error to classify.
    ///
    /// # Returns
    ///
    /// A configuration error retaining `key` and structured source context.
    fn from_value_error(key: &str, error: ValueError) -> Self {
        match error {
            ValueError::TypeMismatch { expected, actual } => Self::TypeMismatch {
                key: key.to_string(),
                expected,
                actual,
            },
            ValueError::Conversion(source) => Self::from_data_conversion_error(key, source),
            ValueError::ListConversion(error) => {
                let (source_index, source) = error.into_parts();
                Self::ConversionError {
                    key: key.to_string(),
                    source_index: Some(source_index),
                    source,
                }
            }
            source => Self::ValueError {
                key: key.to_string(),
                source,
            },
        }
    }

    /// Returns the stable machine-readable category of this error.
    ///
    /// # Returns
    ///
    /// The category corresponding to the concrete error variant.
    pub const fn kind(&self) -> ConfigErrorKind {
        match self {
            Self::InvalidKey { .. } | Self::SourceInvalidKey { .. } => ConfigErrorKind::InvalidKey,
            Self::InvalidPath { .. } => ConfigErrorKind::InvalidPath,
            Self::SourceLimitExceeded { .. } => ConfigErrorKind::SourceLimitExceeded,
            Self::PropertyNotFound(_) => ConfigErrorKind::PropertyNotFound,
            Self::PropertyCandidatesNotFound { .. } => ConfigErrorKind::PropertyNotFound,
            Self::PropertyHasNoValue(_) => ConfigErrorKind::PropertyHasNoValue,
            Self::TypeMismatch { .. } => ConfigErrorKind::TypeMismatch,
            Self::ConversionError { .. } | Self::PlatformIntegerOutOfRange { .. } => ConfigErrorKind::Conversion,
            Self::ValueError { .. } => ConfigErrorKind::Value,
            Self::SubstitutionError { .. } => ConfigErrorKind::Substitution,
            Self::IndexedSubstitutionError { source, .. } => source.kind(),
            Self::SubstitutionDepthExceeded { .. } => ConfigErrorKind::SubstitutionDepthExceeded,
            Self::SubstitutionExpansionLimitExceeded { .. } => ConfigErrorKind::SubstitutionExpansionLimitExceeded,
            Self::SubstitutionOutputTooLarge { .. } => ConfigErrorKind::SubstitutionOutputTooLarge,
            Self::SubstitutionCycle { .. } => ConfigErrorKind::SubstitutionCycle,
            Self::MergeError(_) => ConfigErrorKind::Merge,
            Self::PropertyIsFinal(_) | Self::SourcePropertyIsFinal { .. } => ConfigErrorKind::PropertyIsFinal,
            Self::KeyConflict { .. } => ConfigErrorKind::KeyConflict,
            Self::UnknownProperties { .. } => ConfigErrorKind::UnknownProperty,
            Self::IoError(_) | Self::SourceIoError { .. } => ConfigErrorKind::Io,
            Self::ParseError(_) | Self::SourceParseError { .. } => ConfigErrorKind::Parse,
            Self::DeserializeError { .. } => ConfigErrorKind::Deserialize,
            Self::Other(_) => ConfigErrorKind::Other,
        }
    }

    /// Returns the configuration path carried by this error.
    ///
    /// For structured-read errors, special object member names use
    /// JSON-quoted bracket segments (for example, `payload["a.b"]`) so that
    /// they remain distinct from nested configuration paths. This diagnostic
    /// representation is not a query path for `Config::get` or section lookup.
    ///
    /// # Returns
    ///
    /// `Some(path)` for errors tied to one configuration key, otherwise
    /// `None`.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::InvalidKey { key: path, .. }
            | Self::SourceInvalidKey { key: path, .. }
            | Self::InvalidPath { path, .. }
            | Self::PropertyNotFound(path)
            | Self::PropertyHasNoValue(path)
            | Self::PropertyIsFinal(path) => Some(path),
            Self::SourcePropertyIsFinal { key, .. } => Some(key),
            Self::PropertyCandidatesNotFound { paths } => match paths.as_slice() {
                [path] => Some(path),
                _ => None,
            },
            Self::TypeMismatch { key, .. }
            | Self::ConversionError { key, .. }
            | Self::PlatformIntegerOutOfRange { key, .. }
            | Self::ValueError { key, .. } => Some(key),
            Self::KeyConflict { path, .. }
            | Self::DeserializeError { path, .. }
            | Self::SubstitutionError { path, .. }
            | Self::SubstitutionDepthExceeded { path, .. }
            | Self::SubstitutionExpansionLimitExceeded { path, .. }
            | Self::SubstitutionOutputTooLarge { path, .. }
            | Self::SubstitutionCycle { path, .. } => Some(path),
            Self::IndexedSubstitutionError { source, .. } => source.path(),
            Self::SourceParseError { path: Some(path), .. } => Some(path),
            Self::UnknownProperties { paths } => match paths.as_slice() {
                [path] => Some(path),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the stable source identifier carried by a source-loading error,
    /// a source-originated key conflict, an invalid assignment key from a
    /// source context, or a final-property merge conflict.
    ///
    /// # Returns
    ///
    /// The source path or in-memory source label for source I/O, parse,
    /// resource-limit, source-originated key-conflict, source assignment key,
    /// and final-property merge errors; otherwise `None`.
    #[inline]
    pub fn source_id(&self) -> Option<&str> {
        match self {
            Self::SourceLimitExceeded { source_id, .. }
            | Self::SourceIoError { source_id, .. }
            | Self::SourceParseError { source_id, .. }
            | Self::SourceInvalidKey { source_id, .. }
            | Self::SourcePropertyIsFinal { source_id, .. } => Some(source_id),
            Self::KeyConflict {
                source_id: Some(source_id),
                ..
            } => Some(source_id),
            _ => None,
        }
    }

    /// Returns the physical source location for a source parse error.
    ///
    /// Lines and columns are one-based; the column may be absent when only
    /// the line can be determined reliably.
    #[inline]
    pub const fn source_location(&self) -> Option<SourceLocation> {
        match self {
            Self::SourceParseError { location, .. } => *location,
            _ => None,
        }
    }

    /// Attaches a physical source location to a source parse error.
    ///
    /// Other error variants are returned unchanged.
    ///
    /// # Parameters
    /// - `location`: Physical source location to attach when this is a
    ///   source-parse error.
    ///
    /// # Returns
    /// This error with `location` attached to `SourceParseError`, or the
    /// original error for any other variant.
    pub(crate) fn with_source_location(self, location: SourceLocation) -> Self {
        match self {
            Self::SourceParseError {
                source_id,
                path,
                source_index,
                message,
                ..
            } => Self::SourceParseError {
                source_id,
                path,
                source_index,
                message,
                location: Some(location),
            },
            other => other,
        }
    }

    /// Returns the local or aggregate budget identifier for a source limit
    /// failure.
    ///
    /// # Returns
    ///
    /// Returns the budget identifier for a source-limit error, or `None` for
    /// other error variants.
    #[inline]
    pub fn source_budget_id(&self) -> Option<&str> {
        match self {
            Self::SourceLimitExceeded { budget_id, .. } => Some(budget_id),
            _ => None,
        }
    }

    /// Returns the structured budget failure for a source limit error.
    ///
    /// # Returns
    ///
    /// Returns the borrowed budget failure for a source-limit error, or
    /// `None` otherwise.
    #[inline]
    pub const fn budget_error(&self) -> Option<&BudgetError<SourceLimitKind, usize>> {
        match self {
            Self::SourceLimitExceeded { source, .. } => Some(source),
            _ => None,
        }
    }

    /// Returns missing candidate paths carried by a property lookup error.
    ///
    /// # Returns
    ///
    /// A one-element slice for [`Self::PropertyNotFound`], all ordered
    /// candidates for [`Self::PropertyCandidatesNotFound`], or `None` for
    /// other error kinds.
    #[inline]
    pub fn candidate_paths(&self) -> Option<&[String]> {
        match self {
            Self::PropertyNotFound(path) => Some(std::slice::from_ref(path)),
            Self::PropertyCandidatesNotFound { paths } => Some(paths),
            _ => None,
        }
    }

    /// Returns unknown configuration paths found by strict deserialization.
    ///
    /// Special member names use JSON-quoted bracket segments; for example,
    /// the literal key `a.b` is reported as `payload["a.b"]`, distinct from
    /// the nested path `payload.a.b`. These strings are diagnostic paths, not
    /// configuration lookup paths.
    ///
    /// # Returns
    ///
    /// Returns the unknown-property path list for `UnknownProperties`, or
    /// `None` for other error variants.
    #[inline]
    pub fn unknown_property_paths(&self) -> Option<&[String]> {
        match self {
            Self::UnknownProperties { paths } => Some(paths),
            _ => None,
        }
    }

    /// Returns the failing collection element index when available.
    ///
    /// # Returns
    ///
    /// The original zero-based source index for collection conversion errors,
    /// indexed substitution errors, source parse errors, and indexed value
    /// errors when available; otherwise `None`.
    #[inline(always)]
    pub const fn source_index(&self) -> Option<usize> {
        match self {
            Self::ConversionError { source_index, .. }
            | Self::PlatformIntegerOutOfRange { source_index, .. }
            | Self::SourceParseError { source_index, .. } => *source_index,
            Self::IndexedSubstitutionError { source_index, .. } => Some(*source_index),
            Self::ValueError { source, .. } => match source {
                ValueError::Missing(missing) => missing.source_index(),
                ValueError::JsonProjectionLimit { source_index, .. } => *source_index,
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the original value-layer missing context, when available.
    ///
    /// # Returns
    ///
    /// Returns the borrowed missing-value details from a value error, or
    /// `None` otherwise.
    #[inline]
    pub const fn value_missing(&self) -> Option<&ValueMissing> {
        match self {
            Self::ValueError { source, .. } => source.missing(),
            _ => None,
        }
    }

    /// Adds a source identifier to an invalid assignment-key error.
    ///
    /// Other error variants are returned unchanged.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable source identifier attached to an invalid key.
    ///
    /// # Returns
    ///
    /// Returns a source-aware key error for `InvalidKey`, or the original
    /// variant for every other error.
    pub(crate) fn with_assignment_source(self, source_id: &str) -> Self {
        match self {
            Self::InvalidKey { key, violation } => Self::SourceInvalidKey {
                source_id: source_id.to_owned(),
                key,
                violation,
            },
            error => error,
        }
    }

    /// Adds the incoming source identifier to a final-property merge error.
    ///
    /// Other errors are returned unchanged so source-loading failures retain
    /// their original classification and context.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable identifier of the incoming configuration source.
    ///
    /// # Returns
    ///
    /// Returns `SourcePropertyIsFinal` for a final-property error and the
    /// original error variant otherwise.
    pub(crate) fn with_merge_source(self, source_id: &str) -> Self {
        match self {
            Self::PropertyIsFinal(key) => Self::SourcePropertyIsFinal {
                source_id: source_id.to_owned(),
                key,
            },
            error => error,
        }
    }

    /// Converts a source-originated structural error to a source-aware error.
    ///
    /// # Parameters
    ///
    /// - `source_id`: stable identifier or path for the source that produced
    ///   the error.
    /// - `path`: optional source-relative path for errors converted to parse
    ///   errors.
    /// - `source_index`: optional original collection index for converted parse
    ///   errors.
    ///
    /// # Returns
    ///
    /// Returns a source-aware key conflict or parse error when applicable;
    /// other error variants remain unchanged.
    pub(crate) fn with_source_context(
        self,
        source_id: &str,
        path: Option<String>,
        source_index: Option<usize>,
    ) -> Self {
        match self {
            Self::KeyConflict {
                source_id: None,
                path: conflict_path,
                existing,
                incoming,
            } => Self::KeyConflict {
                source_id: Some(source_id.to_string()),
                path: conflict_path,
                existing,
                incoming,
            },
            Self::InvalidKey { .. } | Self::ParseError(_) => {
                Self::source_parse_error_at(source_id, path, source_index, self.to_string())
            }
            error => error,
        }
    }
}

impl From<(&str, ValueError)> for ConfigError {
    /// Converts a value-layer error with mandatory configuration path context.
    ///
    /// # Parameters
    ///
    /// * `(key, error)` - Configuration path and value-layer error.
    ///
    /// # Returns
    ///
    /// A classified configuration error retaining `key`.
    #[inline]
    fn from((key, error): (&str, ValueError)) -> Self {
        Self::from_value_error(key, error)
    }
}

/// Result type for configuration operations.
///
/// Used for operations that can return a [`ConfigError`].
///
/// # Type Parameters
///
/// - `T`: successful value returned by the configuration operation.
///
/// # Examples
///
/// ```
/// use qubit_config::{Config, ConfigResult};
///
/// fn read_port(config: &Config) -> ConfigResult<i32> {
///     config.get("server.port")
/// }
///
/// assert!(read_port(&Config::new()).is_err());
/// ```
pub type ConfigResult<T> = Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use qubit_datatype::DataConversionError;
    use qubit_datatype::DataType;
    use qubit_datatype::InvalidValueReason;

    use super::ConfigError;
    use crate::SourceLocation;

    #[test]
    fn test_source_parse_location_helpers_preserve_context() {
        let ordinary = ConfigError::source_parse_error("memory:test", "bad syntax");
        assert_eq!(ordinary.source_location(), None);

        let indexed =
            ConfigError::source_parse_error_at("memory:test", Some("a..b".to_string()), Some(3), "bad syntax");
        let location = SourceLocation::new(NonZeroUsize::new(2).unwrap(), Some(NonZeroUsize::new(4).unwrap()));
        let located = indexed.with_source_location(location);
        assert_eq!(located.source_location(), Some(location));
        assert_eq!(located.kind(), crate::ConfigErrorKind::Parse);
        assert_eq!(located.path(), Some("a..b"));
        assert_eq!(located.source_id(), Some("memory:test"));
        assert_eq!(located.source_index(), Some(3));

        assert_eq!(
            ConfigError::ParseError("bad syntax".to_string())
                .with_source_location(location)
                .source_location(),
            None,
        );
    }

    #[test]
    fn test_context_accessors_cover_empty_and_indexed_cases() {
        let no_candidates = ConfigError::PropertyCandidatesNotFound { paths: Vec::new() };
        assert_eq!(no_candidates.path(), None);

        let multiple_unknown = ConfigError::UnknownProperties {
            paths: vec!["first".to_string(), "second".to_string()],
        };
        assert_eq!(multiple_unknown.path(), None);

        let indexed_parse = ConfigError::SourceParseError {
            source_id: "memory:test".to_string(),
            path: None,
            source_index: Some(2),
            message: "invalid element at source index 2".to_string(),
            location: None,
        };
        assert_eq!(indexed_parse.path(), None);
        assert_eq!(indexed_parse.source_index(), Some(2));

        let invalid_value = ConfigError::from_data_conversion_error(
            "value",
            DataConversionError::invalid(DataType::String, DataType::Bool, InvalidValueReason::InvalidBoolean),
        );
        assert_eq!(invalid_value.source_index(), None);
    }
}
