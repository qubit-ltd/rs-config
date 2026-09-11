// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed object-entry iterators used while merging prepared reads.

use std::collections::hash_map;

use super::super::node::ReadNode;

pub(crate) enum MergeEntries<'a> {
    Json(serde_json::map::Iter<'a>),
    StringMap(hash_map::Iter<'a, String, String>),
}

impl<'a> Iterator for MergeEntries<'a> {
    type Item = (&'a str, ReadNode<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Json(values) => values.next().map(|(key, value)| (key.as_str(), ReadNode::Json(value))),
            Self::StringMap(values) => values
                .next()
                .map(|(key, value)| (key.as_str(), ReadNode::Text(std::borrow::Cow::Borrowed(value)))),
        }
    }
}
