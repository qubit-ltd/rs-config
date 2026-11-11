// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Native homogeneous or JSON sequence input.

use qubit_value::MultiValuesRef;

use super::super::traversal::ReadView;

/// Borrowed sequence source extracted from a native collection or JSON array.
pub(in crate::structured_read) enum SequenceValues<'tree> {
    /// Native homogeneous collection.
    Collection(
        /// Borrowed collection data and its scalar elements.
        MultiValuesRef<'tree>,
    ),
    /// Borrowed JSON array.
    Json(
        /// JSON elements retained from the source array.
        &'tree [serde_json::Value],
    ),
}

impl<'tree> SequenceValues<'tree> {
    /// Extracts sequence input, returning `None` for scalar input.
    ///
    /// # Parameters
    ///
    /// * `value` - Read view inspected for native collection or JSON-array
    ///   shape.
    #[inline]
    pub(in crate::structured_read) fn from_view(value: ReadView<'tree>) -> Option<Self> {
        match value {
            ReadView::Collection(values) => Some(Self::Collection(values)),
            ReadView::Json(serde_json::Value::Array(values)) => Some(Self::Json(values)),
            _ => None,
        }
    }

    /// Returns the number of sequence elements.
    #[must_use]
    #[inline]
    pub(in crate::structured_read) fn len(&self) -> usize {
        match self {
            Self::Collection(values) => values.len(),
            Self::Json(values) => values.len(),
        }
    }

    /// Returns the element at `index`, if present.
    ///
    /// # Parameters
    ///
    /// * `index` - Zero-based sequence position to inspect.
    #[inline]
    pub(in crate::structured_read) fn get(&self, index: usize) -> Option<ReadView<'tree>> {
        match self {
            Self::Collection(values) => values.get(index).map(ReadView::from_scalar),
            Self::Json(values) => values.get(index).map(ReadView::Json),
        }
    }
}
