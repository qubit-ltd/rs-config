// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Error type used by the configuration serde deserializer.

use std::fmt;

use serde::de;

use crate::ConfigError;

/// Error produced by the configuration serde deserializer.
#[must_use = "handle or propagate the configuration deserialization error"]
#[derive(Debug)]
pub(crate) enum ConfigDeserializeError {
    /// A serde-originated diagnostic message.
    Message {
        /// Safe serde diagnostic summary retained for public error formatting.
        message: String,
        /// Most specific configuration path reached before the failure.
        path: Option<String>,
    },
    /// A structured configuration error that must retain its kind and leaf
    /// path.
    Config(
        /// Structured error retaining its classification and leaf path.
        ConfigError,
    ),
}

impl ConfigDeserializeError {
    /// Creates a safe diagnostic for a value with the wrong configuration
    /// shape.
    ///
    /// # Parameters
    ///
    /// * `path` - Configuration path at which the value failed validation.
    /// * `expected` - Expected structural shape, such as a sequence or map.
    pub(crate) fn shape_mismatch(path: &str, expected: &'static str) -> Self {
        Self::Message {
            message: format!("configuration value must have shape: {expected}"),
            path: Some(path.to_owned()),
        }
    }

    /// Creates a safe diagnostic for a tuple with a known, incorrect length.
    ///
    /// # Parameters
    ///
    /// * `path` - Configuration path of the tuple.
    /// * `actual` - Number of elements found in the input tuple.
    /// * `expected` - Required number of tuple elements.
    pub(crate) fn tuple_length(path: &str, actual: usize, expected: usize) -> Self {
        Self::Message {
            message: format!("configuration tuple has length {actual}; expected {expected}"),
            path: Some(path.to_owned()),
        }
    }

    /// Creates a safe diagnostic when a scalar sequence has more items than a
    /// tuple accepts.
    ///
    /// # Parameters
    ///
    /// * `path` - Configuration path of the sequence.
    /// * `expected` - Maximum number of elements accepted by the tuple.
    pub(crate) fn tuple_has_extra_items(path: &str, expected: usize) -> Self {
        Self::Message {
            message: format!("configuration tuple has more elements than expected length {expected}"),
            path: Some(path.to_owned()),
        }
    }

    /// Creates a deserialization error from a structured configuration error.
    ///
    /// # Parameters
    ///
    /// * `error` - Structured error whose kind and path must be preserved.
    #[inline]
    pub(crate) fn from_config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    /// Attaches a leaf path to a serde-originated error when it has none.
    ///
    /// # Parameters
    ///
    /// * `path` - Leaf path to use when the error does not already have one.
    pub(crate) fn with_path(self, path: String) -> Self {
        match self {
            Self::Message { message, path: None } => Self::Message {
                message,
                path: Some(path),
            },
            error => error,
        }
    }

    /// Converts this error into the public configuration error type.
    ///
    /// # Parameters
    ///
    /// * `path` - Fallback path used when the stored diagnostic has no path.
    pub(crate) fn into_config_error(self, path: &str) -> ConfigError {
        match self {
            ConfigDeserializeError::Message {
                message,
                path: message_path,
            } => ConfigError::DeserializeError {
                path: message_path.unwrap_or_else(|| path.to_string()),
                message,
                source: None,
            },
            ConfigDeserializeError::Config(error) => error,
        }
    }
}

impl de::Error for ConfigDeserializeError {
    /// Creates a generic safe summary without retaining user-provided text.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Displayable input type supplied by the Serde implementation.
    ///
    /// # Parameters
    ///
    /// * `_msg` - Serde diagnostic, deliberately omitted to avoid retaining
    ///   potentially sensitive input text.
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self::Message {
            message: "configuration value does not match the requested type".to_string(),
            path: None,
        }
    }

    /// Creates a safe summary for a missing schema field.
    ///
    /// # Parameters
    ///
    /// * `field` - Name of the required field absent from the input.
    fn missing_field(field: &'static str) -> Self {
        let field = serde_json::to_string(field).expect("serializing a string cannot fail");
        Self::Message {
            message: format!("missing required field: {field}"),
            path: None,
        }
    }

    /// Creates a safe summary for an invalid value type.
    ///
    /// # Parameters
    ///
    /// * `_unexpected` - Serde description of the value that was found.
    /// * `_expected` - Serde description of the type required by the schema.
    fn invalid_type(_unexpected: de::Unexpected<'_>, _expected: &dyn de::Expected) -> Self {
        Self::Message {
            message: "configuration value has an invalid type".to_string(),
            path: None,
        }
    }

    /// Creates a safe summary for an unknown schema enum variant.
    ///
    /// # Parameters
    ///
    /// * `_variant` - Unrecognized variant name supplied by the input.
    /// * `expected` - Variant names accepted by the schema.
    fn unknown_variant(_variant: &str, expected: &'static [&'static str]) -> Self {
        let expected = serde_json::to_string(expected).expect("serializing variant names cannot fail");
        Self::Message {
            message: format!("unknown configuration variant; expected one of: {expected}"),
            path: None,
        }
    }

    /// Creates a safe summary for an invalid sequence or tuple length.
    ///
    /// # Parameters
    ///
    /// * `len` - Number of sequence elements found in the input.
    /// * `_expected` - Serde description of the expected sequence shape.
    fn invalid_length(len: usize, _expected: &dyn de::Expected) -> Self {
        Self::Message {
            message: format!("configuration value has an invalid length: {len}"),
            path: None,
        }
    }
}

impl fmt::Display for ConfigDeserializeError {
    /// Formats the error message.
    ///
    /// # Parameters
    ///
    /// * `f` - Formatter receiving the safe error message.
    ///
    /// # Returns
    ///
    /// Formatting success or the formatter's error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigDeserializeError::Message { message, .. } => f.write_str(message),
            ConfigDeserializeError::Config(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConfigDeserializeError {
    /// Returns the underlying configuration error when available.
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigDeserializeError::Message { .. } => None,
            ConfigDeserializeError::Config(error) => Some(error),
        }
    }
}
