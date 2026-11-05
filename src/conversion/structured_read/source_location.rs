// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Original property and nested location for interpolation overlays.

use super::source_segment::SourceSegment;

/// Location of a value within one original configuration property.
///
/// `property_index` indexes the property table in `SourceIndex`.
/// `segments` identify nested object keys within that property's structured
/// value. Keeping segments typed prevents a literal dot inside a key from
/// being confused with a separator in a dotted configuration path. Locations
/// are used to associate interpolation overlays with actual string leaves.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SourceLocation {
    /// Index of the originating property.
    pub(super) property_index: usize,
    /// Path segments within that property's structured value.
    pub(super) segments: Vec<SourceSegment>,
}

impl SourceLocation {
    /// Returns a child location by cloning this location and appending one
    /// typed key or array-index segment.
    ///
    /// The property-table index is preserved. Segments remain distinct from
    /// dotted diagnostic paths, so a literal dot in a key is not interpreted
    /// as a path separator.
    ///
    /// # Parameters
    ///
    /// - `segment`: typed child key or array index to append to the nested
    ///   value path.
    ///
    /// # Returns
    ///
    /// Returns a new location with the same property index and the appended
    /// segment; the original location remains unchanged.
    #[must_use]
    pub(super) fn child(&self, segment: SourceSegment) -> Self {
        let mut result = self.clone();
        result.segments.push(segment);
        result
    }
}
