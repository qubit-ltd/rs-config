// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Canonical configuration property keys.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

use crate::config_path::validate_config_key;
use crate::{ConfigError, ConfigResult};

/// Owned canonical configuration property key.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ConfigKey(String);

impl ConfigKey {
    /// Parses a canonical, non-empty configuration property key.
    ///
    /// # Parameters
    ///
    /// * `value` - Candidate dotted key.
    ///
    /// # Returns
    ///
    /// The validated key.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidKey`] when `value` is empty, begins or
    /// ends with `.`, or contains an empty dotted segment.
    pub fn parse(value: impl Into<String>) -> ConfigResult<Self> {
        let value = value.into();
        validate_config_key(&value).map_err(|violation| ConfigError::InvalidKey {
            key: value.clone(),
            violation,
        })?;
        Ok(Self(value))
    }

    /// Borrows the canonical key text.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes this key and returns its owned text.
    #[inline(always)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for ConfigKey {
    /// Deserializes and validates a canonical property key.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl Display for ConfigKey {
    /// Formats the canonical key text.
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ConfigKey {
    /// Borrows the canonical key text.
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
