// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Canonical configuration section paths and shared validation.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ConfigError, ConfigPathViolation, ConfigResult};

/// Owned canonical configuration section path.
///
/// Unlike [`crate::ConfigKey`], the empty string is valid and represents the
/// root configuration scope.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ConfigPath(String);

impl ConfigPath {
    /// Parses a canonical configuration path.
    ///
    /// # Parameters
    ///
    /// * `value` - Candidate dotted path; an empty value represents the root.
    ///
    /// # Returns
    ///
    /// The validated path.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidPath`] when a non-empty value begins or
    /// ends with `.`, or contains an empty dotted segment.
    pub fn parse(value: impl Into<String>) -> ConfigResult<Self> {
        let value = value.into();
        validate_config_path(&value).map_err(|violation| ConfigError::InvalidPath {
            path: value.clone(),
            violation,
        })?;
        Ok(Self(value))
    }

    /// Borrows the canonical path text.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes this path and returns its owned text.
    #[inline(always)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for ConfigPath {
    /// Deserializes and validates a canonical configuration path.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl Display for ConfigPath {
    /// Formats the canonical path text.
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ConfigPath {
    /// Borrows the canonical path text.
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Validates a non-empty configuration property key without allocation.
pub(crate) fn validate_config_key(value: &str) -> Result<(), ConfigPathViolation> {
    if value.is_empty() {
        Err(ConfigPathViolation::Empty)
    } else {
        validate_non_empty_path(value)
    }
}

/// Validates a property key and preserves the rejected input in the error.
pub(crate) fn ensure_config_key(value: &str) -> ConfigResult<()> {
    validate_config_key(value).map_err(|violation| ConfigError::InvalidKey {
        key: value.to_string(),
        violation,
    })
}

/// Validates a configuration path without allocation.
///
/// The empty path is accepted as the root scope.
pub(crate) fn validate_config_path(value: &str) -> Result<(), ConfigPathViolation> {
    if value.is_empty() {
        Ok(())
    } else {
        validate_non_empty_path(value)
    }
}

/// Validates a section path and preserves the rejected input in the error.
pub(crate) fn ensure_config_path(value: &str) -> ConfigResult<()> {
    validate_config_path(value).map_err(|violation| ConfigError::InvalidPath {
        path: value.to_string(),
        violation,
    })
}

/// Validates a non-empty dotted key or path.
fn validate_non_empty_path(value: &str) -> Result<(), ConfigPathViolation> {
    if value.starts_with('.') {
        Err(ConfigPathViolation::LeadingSeparator)
    } else if value.ends_with('.') {
        Err(ConfigPathViolation::TrailingSeparator)
    } else if value.split('.').any(str::is_empty) {
        Err(ConfigPathViolation::EmptySegment)
    } else {
        Ok(())
    }
}
