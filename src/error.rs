/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Error Types
//!
//! Defines all possible error types in the configuration system.
//!
//! # Author
//!
//! Haixing Hu

use thiserror::Error;

use qubit_common::DataType;
use qubit_value::ValueError;

/// Configuration error type
///
/// Defines all possible error scenarios in the configuration system.
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::{Config, ConfigError, ConfigResult};
/// fn get_port(config: &Config) -> ConfigResult<i32> { unimplemented!() }
/// ```
///
/// # Author
///
/// Haixing Hu
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

/// Result type for configuration operations
///
/// Used for all operations in the configuration system that may return errors.
pub type ConfigResult<T> = Result<T, ConfigError>;

impl ConfigError {
    /// Creates a `TypeMismatch` error with an empty key (for backward compatibility with
    /// `ValueError` conversions that don't have key context).
    pub(crate) fn type_mismatch_no_key(expected: DataType, actual: DataType) -> Self {
        ConfigError::TypeMismatch {
            key: String::new(),
            expected,
            actual,
        }
    }

    /// Creates a `TypeMismatch` error with a specific key.
    pub(crate) fn type_mismatch_at(key: &str, expected: DataType, actual: DataType) -> Self {
        ConfigError::TypeMismatch {
            key: key.to_string(),
            expected,
            actual,
        }
    }

    /// Creates a `ConversionError` with an empty key.
    pub(crate) fn conversion_error_no_key(message: impl Into<String>) -> Self {
        ConfigError::ConversionError {
            key: String::new(),
            message: message.into(),
        }
    }

    /// Creates a `ConversionError` with a specific key.
    pub(crate) fn conversion_error_at(key: &str, message: impl Into<String>) -> Self {
        ConfigError::ConversionError {
            key: key.to_string(),
            message: message.into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_error_at_creates_correct_error() {
        let err = ConfigError::conversion_error_at("my.key", "test message");
        match err {
            ConfigError::ConversionError { key, message } => {
                assert_eq!(key, "my.key");
                assert_eq!(message, "test message");
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_type_mismatch_at_creates_correct_error() {
        use qubit_common::DataType;
        let err = ConfigError::type_mismatch_at("a.b", DataType::Bool, DataType::Int32);
        match err {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "a.b");
                assert_eq!(expected, DataType::Bool);
                assert_eq!(actual, DataType::Int32);
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }
}
