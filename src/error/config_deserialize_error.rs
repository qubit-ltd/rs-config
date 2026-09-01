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
#[derive(Debug)]
pub(crate) enum ConfigDeserializeError {
    /// A serde-originated diagnostic message.
    Message {
        /// Original serde diagnostic, retained only for internal formatting.
        message: String,
        /// Most specific configuration path reached before the failure.
        path: Option<String>,
    },
    /// A structured configuration error that must retain its kind and leaf
    /// path.
    Config(ConfigError),
}

impl ConfigDeserializeError {
    /// Creates a deserialization error from a structured configuration error.
    pub(crate) fn from_config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    /// Attaches a leaf path to a serde-originated error when it has none.
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
    pub(crate) fn into_config_error(self, path: &str) -> ConfigError {
        match self {
            ConfigDeserializeError::Message { path: message_path, .. } => ConfigError::DeserializeError {
                path: message_path.unwrap_or_else(|| path.to_string()),
                message: "configuration value does not match the requested type".to_string(),
                source: None,
            },
            ConfigDeserializeError::Config(error) => error,
        }
    }
}

impl de::Error for ConfigDeserializeError {
    /// Creates a custom serde error.
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message {
            message: msg.to_string(),
            path: None,
        }
    }
}

impl fmt::Display for ConfigDeserializeError {
    /// Formats the error message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigDeserializeError::Message { message, .. } => f.write_str(message),
            ConfigDeserializeError::Config(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConfigDeserializeError {
    /// Returns the underlying configuration error when available.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigDeserializeError::Message { .. } => None,
            ConfigDeserializeError::Config(error) => Some(error),
        }
    }
}
