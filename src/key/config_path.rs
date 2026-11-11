// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Canonical configuration section paths and shared validation.

use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error;

use crate::ConfigError;
use crate::ConfigPathViolation;
use crate::ConfigResult;

/// Owned canonical configuration section path.
///
/// Unlike [`crate::ConfigKey`], the empty string is valid and represents the
/// root configuration scope.
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigPath;
///
/// let root = ConfigPath::parse("").unwrap();
/// assert_eq!(root.as_str(), "");
///
/// let server = ConfigPath::parse("server").unwrap();
/// assert_eq!(server.as_str(), "server");
///
/// let error = ConfigPath::parse("server..port").unwrap_err();
/// assert_eq!(error.kind(), qubit_config::ConfigErrorKind::InvalidPath);
/// ```
#[must_use = "retain or inspect the parsed configuration path"]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ConfigPath(
    /// Validated dotted configuration path text.
    String,
);

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
    ///
    /// # Returns
    ///
    /// The path text without allocating.
    #[inline]
    #[must_use = "inspect the canonical path text"]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes this path and returns its owned text.
    ///
    /// # Returns
    ///
    /// The owned canonical path text.
    #[inline]
    #[must_use = "retain or inspect the owned path text"]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for ConfigPath {
    /// Deserializes and validates a canonical configuration path.
    ///
    /// # Type Parameters
    ///
    /// * `D` - Serde deserializer implementation.
    ///
    /// # Parameters
    ///
    /// * `deserializer` - Serde deserializer supplying the path string.
    ///
    /// # Returns
    ///
    /// The validated path, or the deserializer's error.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(Error::custom)
    }
}

impl Display for ConfigPath {
    /// Formats the canonical path text.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination for the path text.
    ///
    /// # Returns
    ///
    /// Formatting success or the destination's formatting error.
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ConfigPath {
    /// Borrows the canonical path text.
    ///
    /// # Returns
    ///
    /// The path text without allocating.
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Validates a non-empty configuration property key without allocation.
///
/// # Parameters
///
/// * `value` - Candidate property key whose dotted segments must be non-empty.
pub(crate) fn validate_config_key(value: &str) -> Result<(), ConfigPathViolation> {
    if value.is_empty() {
        Err(ConfigPathViolation::Empty)
    } else {
        validate_non_empty_path(value)
    }
}

/// Validates a property key and preserves the rejected input in the error.
///
/// # Parameters
///
/// * `value` - Candidate property key to validate and retain in any error.
pub(crate) fn ensure_config_key(value: &str) -> ConfigResult<()> {
    validate_config_key(value).map_err(|violation| ConfigError::InvalidKey {
        key: value.to_string(),
        violation,
    })
}

/// Validates a configuration path without allocation.
///
/// The empty path is accepted as the root scope.
///
/// # Parameters
///
/// * `value` - Candidate dotted section path; empty means the root scope.
pub(crate) fn validate_config_path(value: &str) -> Result<(), ConfigPathViolation> {
    if value.is_empty() {
        Ok(())
    } else {
        validate_non_empty_path(value)
    }
}

/// Validates a section path and preserves the rejected input in the error.
///
/// # Parameters
///
/// * `value` - Candidate section path to validate and retain in any error.
pub(crate) fn ensure_config_path(value: &str) -> ConfigResult<()> {
    validate_config_path(value).map_err(|violation| ConfigError::InvalidPath {
        path: value.to_string(),
        violation,
    })
}

/// Validates a non-empty dotted key or path.
///
/// # Parameters
///
/// * `value` - Non-empty dotted text to check for empty or boundary segments.
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
