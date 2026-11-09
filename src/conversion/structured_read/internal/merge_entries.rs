// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed object-entry iterators used while merging prepared reads.

use super::super::node::ReadNode;
use super::ordered_string_map_entries::OrderedStringMapEntries;

/// Borrows child entries from source object representations during merging.
pub(crate) enum MergeEntries<'a> {
    /// Entries of a borrowed JSON object.
    Json(
        /// Borrowed key/value pairs from the original JSON object.
        serde_json::map::Iter<'a>,
    ),
    /// Entries of a borrowed string map.
    StringMap(
        /// Borrowed key/value pairs from the original string map.
        OrderedStringMapEntries<'a>,
    ),
}

impl<'a> Iterator for MergeEntries<'a> {
    /// Borrowed object key paired with a node view over its original value.
    type Item = (&'a str, ReadNode<'a>);

    /// Returns the next key and a borrowed read node for its value.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Json(values) => values.next().map(|(key, value)| (key.as_str(), ReadNode::Json(value))),
            Self::StringMap(values) => values
                .next()
                .map(|(key, value)| (key.as_str(), ReadNode::Text(std::borrow::Cow::Borrowed(value)))),
        }
    }
}
