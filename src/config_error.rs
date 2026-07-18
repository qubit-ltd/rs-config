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

use thiserror::Error;

use qubit_datatype::{
    DataConversionError,
    DataType,
};
use qubit_value::ValueError;

use crate::ConfigErrorKind;

/// Configuration error type.
#[non_exhaustive]
#[must_use]
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Property not found.
    #[error("Property not found: {0}")]
    PropertyNotFound(
        /// Missing configuration path.
        String,
    ),

    /// Property has no value.
    #[error("Property '{0}' has no value")]
    PropertyHasNoValue(
        /// Configuration path whose property has no usable value.
        String,
    ),

    /// Type mismatch at a specific key/path.
    #[error("Type mismatch at '{key}': expected {expected}, actual {actual}")]
    TypeMismatch {
        /// Configuration key/path where the mismatch occurred.
        key: String,
        /// Expected type.
        expected: DataType,
        /// Actual type.
        actual: DataType,
    },

    /// Structured, value-redacted conversion failure.
    #[error("Type conversion failed at '{key}': {source}")]
    ConversionError {
        /// Configuration key/path where the conversion failed.
        key: String,
        /// Zero-based index in the original source collection, when relevant.
        source_index: Option<usize>,
        /// Value-free conversion failure details.
        #[source]
        source: DataConversionError,
    },

    /// Value-layer error not covered by a specialized configuration mapping.
    #[error("Value error at '{key}': {source}")]
    ValueError {
        /// Configuration key/path where the value access failed.
        key: String,
        /// Original value-layer error.
        #[source]
        source: ValueError,
    },

    /// Variable substitution failed.
    #[error("Variable substitution failed at '{path}': {message}")]
    SubstitutionError {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Human-readable resolution failure.
        message: String,
    },

    /// Variable substitution depth exceeded.
    #[error(
        "Variable substitution at '{path}' exceeded maximum depth: {max_depth}"
    )]
    SubstitutionDepthExceeded {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Maximum permitted recursive expansion depth.
        max_depth: usize,
    },

    /// Variable substitution cycle detected.
    #[error(
        "Variable substitution cycle at '{path}': {}",
        chain.join(" -> ")
    )]
    SubstitutionCycle {
        /// Configuration path whose value was being expanded.
        path: String,
        /// Variable chain that forms the cycle.
        chain: Vec<String>,
    },

    /// Configuration merge failed.
    #[error("Configuration merge failed: {0}")]
    MergeError(
        /// Human-readable merge failure.
        String,
    ),

    /// Property is final and cannot be overridden.
    #[error("Property '{0}' is final and cannot be overridden")]
    PropertyIsFinal(
        /// Configuration path protected against mutation.
        String,
    ),

    /// Configuration key path cannot be represented without ambiguity.
    #[error(
        "Configuration key conflict at '{path}': existing {existing}, incoming {incoming}"
    )]
    KeyConflict {
        /// Conflicting configuration key/path.
        path: String,
        /// Existing value or path shape.
        existing: String,
        /// Incoming value or path shape.
        incoming: String,
    },

    /// I/O error.
    #[error("IO error: {0}")]
    IoError(
        /// Underlying I/O failure.
        #[from]
        std::io::Error,
    ),

    /// Parse error.
    #[error("Parse error: {0}")]
    ParseError(
        /// Human-readable source parsing failure.
        String,
    ),

    /// Deserialization error for structured config mapping.
    #[error("Deserialization error at '{path}': {message}")]
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
    #[error("Configuration error: {0}")]
    Other(
        /// Human-readable uncategorized failure.
        String,
    ),
}

impl ConfigError {
    /// Returns the stable machine-readable category of this error.
    ///
    /// # Returns
    ///
    /// The category corresponding to the concrete error variant.
    #[inline(always)]
    pub const fn kind(&self) -> ConfigErrorKind {
        match self {
            Self::PropertyNotFound(_) => ConfigErrorKind::PropertyNotFound,
            Self::PropertyHasNoValue(_) => ConfigErrorKind::PropertyHasNoValue,
            Self::TypeMismatch { .. } => ConfigErrorKind::TypeMismatch,
            Self::ConversionError { .. } => ConfigErrorKind::Conversion,
            Self::ValueError { .. } => ConfigErrorKind::Value,
            Self::SubstitutionError { .. } => ConfigErrorKind::Substitution,
            Self::SubstitutionDepthExceeded { .. } => {
                ConfigErrorKind::SubstitutionDepthExceeded
            }
            Self::SubstitutionCycle { .. } => {
                ConfigErrorKind::SubstitutionCycle
            }
            Self::MergeError(_) => ConfigErrorKind::Merge,
            Self::PropertyIsFinal(_) => ConfigErrorKind::PropertyIsFinal,
            Self::KeyConflict { .. } => ConfigErrorKind::KeyConflict,
            Self::IoError(_) => ConfigErrorKind::Io,
            Self::ParseError(_) => ConfigErrorKind::Parse,
            Self::DeserializeError { .. } => ConfigErrorKind::Deserialize,
            Self::Other(_) => ConfigErrorKind::Other,
        }
    }

    /// Returns the configuration path carried by this error.
    ///
    /// # Returns
    ///
    /// `Some(path)` for errors tied to one configuration key, otherwise
    /// `None`.
    #[inline(always)]
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::PropertyNotFound(path)
            | Self::PropertyHasNoValue(path)
            | Self::PropertyIsFinal(path) => Some(path),
            Self::TypeMismatch { key, .. }
            | Self::ConversionError { key, .. }
            | Self::ValueError { key, .. } => Some(key),
            Self::KeyConflict { path, .. }
            | Self::DeserializeError { path, .. }
            | Self::SubstitutionError { path, .. }
            | Self::SubstitutionDepthExceeded { path, .. }
            | Self::SubstitutionCycle { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Returns the failing collection element index when available.
    ///
    /// # Returns
    ///
    /// The original zero-based source index for collection conversion errors,
    /// otherwise `None`.
    #[inline(always)]
    pub const fn source_index(&self) -> Option<usize> {
        match self {
            Self::ConversionError { source_index, .. } => *source_index,
            _ => None,
        }
    }

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
    pub fn from_data_conversion_error(
        key: &str,
        error: DataConversionError,
    ) -> Self {
        if error.is_missing() {
            Self::PropertyHasNoValue(key.to_string())
        } else {
            Self::ConversionError {
                key: key.to_string(),
                source_index: None,
                source: error,
            }
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
            ValueError::NoValue => Self::PropertyHasNoValue(key.to_string()),
            ValueError::TypeMismatch { expected, actual } => {
                Self::TypeMismatch {
                    key: key.to_string(),
                    expected,
                    actual,
                }
            }
            ValueError::DataConversion(source) => {
                Self::from_data_conversion_error(key, source)
            }
            ValueError::DataListConversion(error) => {
                let source_index = error.source_index();
                Self::ConversionError {
                    key: key.to_string(),
                    source_index: Some(source_index),
                    source: error.into_conversion_error(),
                }
            }
            source => Self::ValueError {
                key: key.to_string(),
                source,
            },
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
