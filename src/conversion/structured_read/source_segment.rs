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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SourceSegment {
    Key(String),
    Index(usize),
}
