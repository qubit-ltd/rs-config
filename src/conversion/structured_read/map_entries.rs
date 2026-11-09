// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Native borrowed object entries for configuration deserialization.
//!
//! Entries borrow keys and payloads from the prepared tree or source value.
//! Origin metadata is cloned only when interpolation diagnostics need it or
//! the value is missing; ordinary reads avoid carrying unused locations.

use super::deserializer::ReadContext;
use super::internal::ordered_string_map_entries::OrderedStringMapEntries;
use super::node::NodeId;
use super::node::SourceLocation;
use super::traversal::ReadView;

/// Borrowed entry iterators over supported map-shaped read views.
///
/// Keys and payloads remain borrowed from their prepared or source container;
/// prepared-node origins are carried only when diagnostics may need them.
pub(super) enum MapEntries<'tree> {
    /// Borrowed keys and child IDs from the prepared object's ordered index.
    Object(
        /// Key and child-node ID pairs from the prepared object.
        std::collections::btree_map::Iter<'tree, String, NodeId>,
    ),
    /// Borrowed entries from an input JSON object.
    Json(
        /// Key and value pairs borrowed from the input object.
        serde_json::map::Iter<'tree>,
    ),
    /// Borrowed key/value pairs from a string map.
    StringMap(
        /// Key and value pairs borrowed from the input string map.
        OrderedStringMapEntries<'tree>,
    ),
}

impl<'tree> MapEntries<'tree> {
    /// Creates borrowed entry access for a supported map-shaped read view.
    ///
    /// Prepared objects, JSON objects, and string maps are supported. Other
    /// shapes return `None`, allowing the caller to report a shape mismatch.
    /// The iterator borrows keys and values from the input; prepared object
    /// entries retain their node IDs for looking up values and optional source
    /// locations.
    ///
    /// # Parameters
    ///
    /// * `value` - Read view to expose when it has a supported map shape.
    pub(super) fn new(value: ReadView<'tree>) -> Option<Self> {
        match value {
            ReadView::Object(values) => Some(Self::Object(values.iter())),
            ReadView::StringMap(values) => Some(Self::StringMap(OrderedStringMapEntries::new(values))),
            ReadView::Json(serde_json::Value::Object(values)) => Some(Self::Json(values.iter())),
            _ => None,
        }
    }

    /// Returns the number of entries not yet yielded by this iterator.
    #[must_use]
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Object(values) => values.len(),
            Self::Json(values) => values.len(),
            Self::StringMap(values) => values.len(),
        }
    }

    /// Returns the next borrowed key, value view, and optional source location.
    ///
    /// `None` means the entry iterator is exhausted. Keys and payloads remain
    /// borrowed from their backing object. A prepared entry clones its source
    /// location only when diagnostics or missing-value recovery may use it;
    /// direct JSON and string-map entries have no prepared source location and
    /// return `None` for that component.
    ///
    /// # Parameters
    ///
    /// * `context` - Prepared read context used to resolve IDs and origins for
    ///   indexed object entries.
    pub(super) fn next(
        &mut self,
        context: ReadContext<'tree>,
    ) -> Option<(&'tree str, ReadView<'tree>, Option<SourceLocation>)> {
        // Entry text and values stay borrowed. Prepared nodes carry their
        // origin only when overlays or missing-value recovery can use it;
        // direct JSON and string-map entries have no prepared origin here.
        match self {
            Self::Object(values) => values.next().map(|(key, id)| {
                let value = ReadView::from_node(&context.prepared.nodes[*id]);
                let origin = if context.needs_locations() || matches!(value, ReadView::Missing(_)) {
                    context.prepared.sources.origin(*id).cloned()
                } else {
                    None
                };
                (key.as_str(), value, origin)
            }),
            Self::Json(values) => values
                .next()
                .map(|(key, value)| (key.as_str(), ReadView::Json(value), None)),
            Self::StringMap(values) => values
                .next()
                .map(|(key, value)| (key.as_str(), ReadView::Text(value), None)),
        }
    }
}
