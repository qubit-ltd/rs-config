// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Typed source segments distinguish object keys from array indices.
//!
//! These identify segments in the original value tree, not pieces of a
//! dot-separated configuration path. Thus an object key containing `.` stays
//! one key here, while a dotted configuration key represents multiple path
//! segments.

/// Typed segments keep a literal dotted JSON key distinct from nested keys.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::structured_read) enum SourceSegment {
    /// Segment for one object key, kept literal even when it contains dots.
    Key(
        /// Original key text from the source value.
        String,
    ),
    /// Segment for an array element in the original value tree.
    Index(
        /// Zero-based position of the element in its source array.
        usize,
    ),
}
