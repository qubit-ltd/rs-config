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
use super::node::NodeId;
use super::node::SourceLocation;
use super::traversal::ReadView;

/// Borrowed entry iterators over supported map-shaped read views.
///
/// Keys and payloads remain borrowed from their prepared or source container;
/// prepared-node origins are carried only when diagnostics may need them.
pub(super) enum MapEntries<'tree> {
    /// Borrowed keys and child IDs from the prepared object's ordered index.
    Object(std::collections::btree_map::Iter<'tree, String, NodeId>),
    /// Borrowed entries from an input JSON object.
    Json(serde_json::map::Iter<'tree>),
    /// Borrowed key/value pairs from a string map.
    StringMap(std::collections::hash_map::Iter<'tree, String, String>),
}

impl<'tree> MapEntries<'tree> {
    pub(super) fn new(value: ReadView<'tree>) -> Option<Self> {
        match value {
            ReadView::Object(values) => Some(Self::Object(values.iter())),
            ReadView::StringMap(values) => Some(Self::StringMap(values.iter())),
            ReadView::Json(serde_json::Value::Object(values)) => Some(Self::Json(values.iter())),
            _ => None,
        }
    }

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

    /// Returns the number of entries not yet yielded by this iterator.
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Object(values) => values.len(),
            Self::Json(values) => values.len(),
            Self::StringMap(values) => values.len(),
        }
    }
}
