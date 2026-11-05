// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Child iterators for each supported prepared-read container.
//!
//! Items borrow their keys and values from the input or prepared arena. The
//! prepared-object variant resolves child IDs together with the source table,
//! so each yielded origin remains aligned with its arena node.

use std::collections::btree_map;
use std::collections::hash_map;

use qubit_value::MultiValuesRef;

use super::super::node::NodeId;
use super::super::prepared_read::PreparedConfigRead;
use super::super::read_view::ReadView;
use super::child::Child;
use super::segment::Segment;

/// Lazily visits direct children of a prepared or source-backed value.
pub(crate) enum ChildIter<'a> {
    /// Ordered children of an indexed prepared object.
    Object {
        /// Ordered key-to-node entries still waiting to be visited.
        values: btree_map::Iter<'a, String, NodeId>,
        /// Prepared arena used to resolve each child node and its origin.
        prepared: &'a PreparedConfigRead<'a>,
    },
    /// Elements of a native homogeneous collection.
    Collection {
        /// Borrowed collection elements.
        values: MultiValuesRef<'a>,
        /// Index of the next collection element to visit.
        next: usize,
    },
    /// Elements of a borrowed JSON array, paired with their source indexes.
    JsonArray(
        /// JSON elements paired with their zero-based source positions.
        std::iter::Enumerate<std::slice::Iter<'a, serde_json::Value>>,
    ),
    /// Entries of a borrowed JSON object.
    JsonObject(
        /// Borrowed key and value pairs from the source object.
        serde_json::map::Iter<'a>,
    ),
    /// Entries of a borrowed string map.
    StringMap(
        /// Borrowed key and value pairs from the source string map.
        hash_map::Iter<'a, String, String>,
    ),
}

impl<'a> Iterator for ChildIter<'a> {
    /// Borrowed child view with its logical path segment and source origin.
    type Item = Child<'a>;

    /// Returns the next child with its logical segment and optional origin.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Object { values, prepared } => values.next().map(|(key, id)| Child {
                segment: Segment::Key(key),
                value: ReadView::from_node(&prepared.nodes[*id]),
                origin: prepared.sources.origin(*id),
            }),
            Self::Collection { values, next } => {
                let index = *next;
                let value = values.get(index)?;
                *next = next.saturating_add(1);
                Some(Child {
                    segment: Segment::Index(index),
                    value: ReadView::from_scalar(value),
                    origin: None,
                })
            }
            Self::JsonArray(values) => values.next().map(|(index, value)| Child {
                segment: Segment::Index(index),
                value: ReadView::Json(value),
                origin: None,
            }),
            Self::JsonObject(values) => values.next().map(|(key, value)| Child {
                segment: Segment::Key(key),
                value: ReadView::Json(value),
                origin: None,
            }),
            Self::StringMap(values) => values.next().map(|(key, value)| Child {
                segment: Segment::Key(key),
                value: ReadView::Text(value),
                origin: None,
            }),
        }
    }
}
