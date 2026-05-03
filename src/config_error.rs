/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Error Type
//!
//! Defines all possible error scenarios in the configuration system.
//!

use thiserror::Error;

use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_value::ValueError;

/// Configuration error type
///
/// Defines all possible error scenarios in the configuration system.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigError, ConfigResult};
/// fn get_port(config: &Config) -> ConfigResult<i32> { unimplemented!() }
/// ```
///
///
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Property not found
    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    /// Property has no value
    #[error("Property '{0}' has no value")]
    PropertyHasNoValue(String),

    /// Type mismatch at a specific key/path
    #[error("Type mismatch at '{key}': expected {expected}, actual {actual}")]
    TypeMismatch {
        /// The configuration key/path where the mismatch occurred
        key: String,
        /// Expected type
        expected: DataType,
        /// Actual type
        actual: DataType,
    },

    /// Type conversion failed at a specific key/path
    #[error("Type conversion failed at '{key}': {message}")]
    ConversionError {
        /// The configuration key/path where the conversion failed
        key: String,
        /// Error message
        message: String,
    },

    /// Index out of bounds
    #[error("Index out of bounds: index {index}, length {len}")]
    IndexOutOfBounds {
        /// Index being accessed
        index: usize,
        /// Actual length
        len: usize,
    },

    /// Variable substitution failed
    #[error("Variable substitution failed: {0}")]
    SubstitutionError(String),

    /// Variable substitution depth exceeded
    #[error("Variable substitution depth exceeded maximum limit: {0}")]
    SubstitutionDepthExceeded(usize),

    /// Variable substitution cycle detected
    #[error("Variable substitution cycle detected: {}", chain.join(" -> "))]
    SubstitutionCycle {
        /// Variable chain that forms the cycle
        chain: Vec<String>,
    },

    /// Configuration merge failed
    #[error("Configuration merge failed: {0}")]
    MergeError(String),

    /// Property is final and cannot be overridden
    #[error("Property '{0}' is final and cannot be overridden")]
    PropertyIsFinal(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Deserialization error for structured config mapping
    #[error("Deserialization error at '{path}': {message}")]
    DeserializeError {
        /// The config prefix/path being deserialized
        path: String,
        /// Error message
        message: String,
    },

    /// Other error
    #[error("Configuration error: {0}")]
    Other(String),
}

impl ConfigError {
    /// Creates a `TypeMismatch` error with an empty key (for backward
    /// compatibility with `ValueError` conversions that lack key context).
    ///
    /// # Parameters
    ///
    /// * `expected` - Expected [`DataType`].
    /// * `actual` - Actual [`DataType`].
    ///
    /// # Returns
    ///
    /// A [`ConfigError::TypeMismatch`] with an empty `key`.
    #[inline]
    pub(crate) fn type_mismatch_no_key(expected: DataType, actual: DataType) -> Self {
        ConfigError::TypeMismatch {
            key: String::new(),
            expected,
            actual,
        }
    }

    /// Creates a `ConversionError` with an empty key.
    ///
    /// # Parameters
    ///
    /// * `message` - Human-readable conversion error message.
    ///
    /// # Returns
    ///
    /// A [`ConfigError::ConversionError`] with an empty `key`.
    #[inline]
    pub(crate) fn conversion_error_no_key(message: impl Into<String>) -> Self {
        ConfigError::ConversionError {
            key: String::new(),
            message: message.into(),
        }
    }

    /// Maps a common data conversion error to a keyed configuration error.
    ///
    /// # Parameters
    ///
    /// * `key` - Configuration key that was being parsed.
    /// * `err` - Error returned by the common conversion layer.
    ///
    /// # Returns
    ///
    /// A [`ConfigError`] carrying the supplied key.
    pub fn from_data_conversion_error(key: &str, err: DataConversionError) -> Self {
        match err {
            DataConversionError::NoValue => ConfigError::PropertyHasNoValue(key.to_string()),
            DataConversionError::ConversionFailed { from, to } => ConfigError::ConversionError {
                key: key.to_string(),
                message: format!("From {from} to {to}"),
            },
            DataConversionError::ConversionError(message) => ConfigError::ConversionError {
                key: key.to_string(),
                message,
            },
            DataConversionError::JsonSerializationError(message) => ConfigError::ConversionError {
                key: key.to_string(),
                message: format!("JSON serialization error: {message}"),
            },
            DataConversionError::JsonDeserializationError(message) => {
                ConfigError::ConversionError {
                    key: key.to_string(),
                    message: format!("JSON deserialization error: {message}"),
                }
            }
        }
    }
}

impl From<ValueError> for ConfigError {
    fn from(err: ValueError) -> Self {
        match err {
            ValueError::NoValue => ConfigError::PropertyHasNoValue(String::new()),
            ValueError::TypeMismatch { expected, actual } => {
                ConfigError::type_mismatch_no_key(expected, actual)
            }
            ValueError::ConversionFailed { from, to } => {
                ConfigError::conversion_error_no_key(format!("From {from} to {to}"))
            }
            ValueError::ConversionError(msg) => ConfigError::conversion_error_no_key(msg),
            ValueError::IndexOutOfBounds { index, len } => {
                ConfigError::IndexOutOfBounds { index, len }
            }
            ValueError::JsonSerializationError(msg) => {
                ConfigError::conversion_error_no_key(format!("JSON serialization error: {msg}"))
            }
            ValueError::JsonDeserializationError(msg) => {
                ConfigError::conversion_error_no_key(format!("JSON deserialization error: {msg}"))
            }
        }
    }
}

impl From<(&str, ValueError)> for ConfigError {
    fn from((key, err): (&str, ValueError)) -> Self {
        match err {
            ValueError::NoValue => ConfigError::PropertyHasNoValue(key.to_string()),
            ValueError::TypeMismatch { expected, actual } => ConfigError::TypeMismatch {
                key: key.to_string(),
                expected,
                actual,
            },
            ValueError::ConversionFailed { from, to } => ConfigError::ConversionError {
                key: key.to_string(),
                message: format!("From {from} to {to}"),
            },
            ValueError::ConversionError(message) => ConfigError::ConversionError {
                key: key.to_string(),
                message,
            },
            ValueError::IndexOutOfBounds { index, len } => {
                ConfigError::IndexOutOfBounds { index, len }
            }
            ValueError::JsonSerializationError(message) => ConfigError::ConversionError {
                key: key.to_string(),
                message: format!("JSON serialization error: {message}"),
            },
            ValueError::JsonDeserializationError(message) => ConfigError::ConversionError {
                key: key.to_string(),
                message: format!("JSON deserialization error: {message}"),
            },
        }
    }
}
