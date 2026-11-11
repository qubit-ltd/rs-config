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
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use qubit_config::SourceLocation;
///
/// let location = SourceLocation::new(
///     NonZeroUsize::new(3).unwrap(),
///     Some(NonZeroUsize::new(7).unwrap()),
/// );
/// assert_eq!(location.line(), 3);
/// assert_eq!(location.column(), Some(7));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceLocation {
    /// One-based physical line number.
    line: NonZeroUsize,
    /// One-based physical column number when the parser can determine it.
    column: Option<NonZeroUsize>,
}

impl SourceLocation {
    /// Creates a source location from one-based line and optional column
    /// values.
    ///
    /// # Parameters
    /// - `line`: Non-zero physical line number.
    /// - `column`: Optional non-zero Unicode-scalar column number.
    #[inline]
    pub const fn new(line: NonZeroUsize, column: Option<NonZeroUsize>) -> Self {
        Self { line, column }
    }

    /// Returns the one-based physical line number.
    #[must_use]
    #[inline]
    pub const fn line(&self) -> usize {
        self.line.get()
    }

    /// Returns the one-based physical column number when known, or `None` if
    /// only the line could be determined reliably.
    #[inline]
    pub const fn column(&self) -> Option<usize> {
        match self.column {
            Some(column) => Some(column.get()),
            None => None,
        }
    }
}
