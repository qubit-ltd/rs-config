// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Canonical configuration path validation failures.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;

/// Describes why a configuration key or path is not canonical.
#[non_exhaustive]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigPathViolation {
    /// A property key is empty.
    Empty,
    /// The value starts with a `.` separator.
    LeadingSeparator,
    /// The value ends with a `.` separator.
    TrailingSeparator,
    /// The value contains an empty segment between two `.` separators.
    EmptySegment,
}

impl Display for ConfigPathViolation {
    /// Formats a stable, value-free description of the violation.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("the key is empty"),
            Self::LeadingSeparator => {
                formatter.write_str("the path starts with a separator")
            }
            Self::TrailingSeparator => {
                formatter.write_str("the path ends with a separator")
            }
            Self::EmptySegment => {
                formatter.write_str("the path contains an empty segment")
            }
        }
    }
}
