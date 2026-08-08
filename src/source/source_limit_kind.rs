// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Configuration source resource dimensions.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;

/// Resource dimension enforced during source ingestion.
#[non_exhaustive]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceLimitKind {
    /// Raw input byte count.
    InputBytes,
    /// Parsed or emitted assignment count.
    PropertyCount,
    /// Root-relative structured nesting depth.
    NestingDepth,
}

impl Display for SourceLimitKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputBytes => formatter.write_str("input bytes"),
            Self::PropertyCount => formatter.write_str("property count"),
            Self::NestingDepth => formatter.write_str("nesting depth"),
        }
    }
}
