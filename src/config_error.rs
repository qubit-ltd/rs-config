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

/// Configuration error type.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Property not found.
    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    /// Property has no value.
    #[error("Property '{0}' has no value")]
    PropertyHasNoValue(String),

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

    /// Variable substitution failed.
    #[error("Variable substitution failed: {0}")]
    SubstitutionError(String),

    /// Variable substitution depth exceeded.
    #[error("Variable substitution depth exceeded maximum limit: {0}")]
    SubstitutionDepthExceeded(usize),

    /// Variable substitution cycle detected.
    #[error("Variable substitution cycle detected: {}", chain.join(" -> "))]
    SubstitutionCycle {
        /// Variable chain that forms the cycle.
        chain: Vec<String>,
    },

    /// Configuration merge failed.
    #[error("Configuration merge failed: {0}")]
    MergeError(String),

    /// Property is final and cannot be overridden.
    #[error("Property '{0}' is final and cannot be overridden")]
    PropertyIsFinal(String),

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
    IoError(#[from] std::io::Error),

    /// Parse error.
    #[error("Parse error: {0}")]
    ParseError(String),

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
    Other(String),
}

impl ConfigError {
    /// Maps a common data conversion error to a keyed configuration error.
    #[inline]
    pub fn from_data_conversion_error(
        key: &str,
        error: DataConversionError,
    ) -> Self {
        if matches!(error, DataConversionError::Missing { .. }) {
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
            ValueError::DataListConversion(error) => Self::ConversionError {
                key: key.to_string(),
                source_index: Some(error.source_index),
                source: error.source,
            },
        }
    }
}

impl From<ValueError> for ConfigError {
    #[inline]
    fn from(error: ValueError) -> Self {
        Self::from_value_error("", error)
    }
}

impl From<(&str, ValueError)> for ConfigError {
    #[inline]
    fn from((key, error): (&str, ValueError)) -> Self {
        Self::from_value_error(key, error)
    }
}
