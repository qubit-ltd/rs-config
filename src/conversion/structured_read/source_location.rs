// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Original property and nested location for interpolation overlays.

use super::source_segment::SourceSegment;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SourceLocation {
    pub(super) property_index: usize,
    pub(super) segments: Vec<SourceSegment>,
}

impl SourceLocation {
    pub(super) fn child(&self, segment: SourceSegment) -> Self {
        let mut result = self.clone();
        result.segments.push(segment);
        result
    }
}
