// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Canonical configuration property keys.

use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error;

use crate::ConfigError;
use crate::ConfigResult;
use crate::config_path::validate_config_key;

/// Owned canonical configuration property key.
///
/// A key is non-empty and uses non-empty segments separated by single dots.
/// It can be passed directly to configuration APIs through its string form.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigKey};
///
/// let key = ConfigKey::parse("server.port")?;
/// assert_eq!(key.as_str(), "server.port");
/// assert!(ConfigKey::parse("server..port").is_err());
///
/// let mut config = Config::new();
/// config.set("server.port", 8080)?;
/// config.set("legacy.port", 80)?;
/// let selected: i32 = config.get_any(["server.port", "legacy.port"])?;
/// assert_eq!(selected, 8080);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[must_use = "use the returned value"]
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
    #[must_use = "use the returned value"]
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
        Self::parse(value).map_err(Error::custom)
    }
}

impl Display for ConfigKey {
    /// Formats the canonical key text.
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ConfigKey {
    /// Borrows the canonical key text.
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
