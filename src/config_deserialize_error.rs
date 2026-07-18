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
    Message(String),
    /// A structured configuration error that must retain its kind and leaf
    /// path.
    Config(ConfigError),
}

impl ConfigDeserializeError {
    /// Creates a deserialization error from a structured configuration error.
    pub(crate) fn from_config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    /// Converts this error into the public configuration error type.
    pub(crate) fn into_config_error(self, path: &str) -> ConfigError {
        match self {
            ConfigDeserializeError::Message(_) => {
                ConfigError::DeserializeError {
                    path: path.to_string(),
                    message:
                        "configuration value does not match the requested type"
                            .to_string(),
                    source: None,
                }
            }
            ConfigDeserializeError::Config(error) => error,
        }
    }
}

impl de::Error for ConfigDeserializeError {
    /// Creates a custom serde error.
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl fmt::Display for ConfigDeserializeError {
    /// Formats the error message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigDeserializeError::Message(message) => f.write_str(message),
            ConfigDeserializeError::Config(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConfigDeserializeError {
    /// Returns the underlying configuration error when available.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigDeserializeError::Message(_) => None,
            ConfigDeserializeError::Config(error) => Some(error),
        }
    }
}
