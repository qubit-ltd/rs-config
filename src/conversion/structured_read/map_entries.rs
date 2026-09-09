// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Native borrowed object entries for configuration deserialization.

use super::deserializer::ReadContext;
use super::node::NodeId;
use super::node::SourceLocation;
use super::traversal::ReadView;

pub(super) enum MapEntries<'tree> {
    Object(std::collections::btree_map::Iter<'tree, String, NodeId>),
    Json(serde_json::map::Iter<'tree>),
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

    pub(super) fn len(&self) -> usize {
        match self {
            Self::Object(values) => values.len(),
            Self::Json(values) => values.len(),
            Self::StringMap(values) => values.len(),
        }
    }
}
