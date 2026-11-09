// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Structured physical source coordinates for configuration parse errors.

use std::num::NonZeroUsize;

/// A one-based physical line and optional one-based column in a config source.
///
/// A missing column indicates that only the line could be determined reliably.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceLocation {
    line: NonZeroUsize,
    column: Option<NonZeroUsize>,
}

impl SourceLocation {
    /// Creates a source location from one-based line and optional column values.
    #[inline]
    pub const fn new(line: NonZeroUsize, column: Option<NonZeroUsize>) -> Self {
        Self { line, column }
    }

    /// Returns the one-based physical line number.
    #[inline]
    pub const fn line(self) -> usize {
        self.line.get()
    }

    /// Returns the one-based physical column number, when known.
    #[inline]
    pub const fn column(self) -> Option<usize> {
        match self.column {
            Some(column) => Some(column.get()),
            None => None,
        }
    }
}
