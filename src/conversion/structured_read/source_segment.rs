// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Typed source segments distinguish object keys from array indices.

/// Typed segments keep a literal dotted JSON key distinct from nested keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SourceSegment {
    Key(String),
    Index(usize),
}
