/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
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
//! Hu Haixing

use thiserror::Error;

use prism3_core::DataType;
use prism3_value::ValueError;

/// Configuration error type
///
/// Defines all possible error scenarios in the configuration system.
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_config::{Config, ConfigError, ConfigResult};
/// fn get_port(config: &Config) -> ConfigResult<i32> { unimplemented!() }
/// ```
///
/// # Author
///
/// Hu Haixing
///
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Property not found
    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    /// Property has no value
    #[error("Property '{0}' has no value")]
    PropertyHasNoValue(String),

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, actual {actual}")]
    TypeMismatch {
        /// Expected type
        expected: DataType,
        /// Actual type
        actual: DataType,
    },

    /// Type conversion failed
    #[error("Type conversion failed: {0}")]
    ConversionError(String),

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

    /// Other error
    #[error("Configuration error: {0}")]
    Other(String),
}

/// Result type for configuration operations
///
/// Used for all operations in the configuration system that may return errors.
pub type ConfigResult<T> = Result<T, ConfigError>;

impl From<ValueError> for ConfigError {
    fn from(err: ValueError) -> Self {
        match err {
            ValueError::NoValue => ConfigError::PropertyHasNoValue("".to_string()),
            ValueError::TypeMismatch { expected, actual } => {
                ConfigError::TypeMismatch { expected, actual }
            }
            ValueError::ConversionFailed { from, to } => {
                ConfigError::ConversionError(format!("From {from} to {to}"))
            }
            ValueError::ConversionError(msg) => ConfigError::ConversionError(msg),
            ValueError::IndexOutOfBounds { index, len } => {
                ConfigError::IndexOutOfBounds { index, len }
            }
        }
    }
}
