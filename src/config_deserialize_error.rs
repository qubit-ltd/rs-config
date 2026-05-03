/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Error type used by the configuration serde deserializer.

use std::fmt;

use serde::de;

/// Error produced by the configuration serde deserializer.
#[derive(Debug)]
pub(crate) struct ConfigDeserializeError {
    message: String,
}

impl de::Error for ConfigDeserializeError {
    /// Creates a custom serde error.
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

impl fmt::Display for ConfigDeserializeError {
    /// Formats the error message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigDeserializeError {}
