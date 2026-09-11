// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Child iterators for each supported prepared-read container.

use std::collections::btree_map;
use std::collections::hash_map;

use qubit_value::MultiValuesRef;

use super::super::node::NodeId;
use super::super::prepared_read::PreparedConfigRead;
use super::super::read_view::ReadView;
use super::child::Child;
use super::segment::Segment;

pub(crate) enum ChildIter<'a> {
    Object {
        values: btree_map::Iter<'a, String, NodeId>,
        prepared: &'a PreparedConfigRead<'a>,
    },
    Collection {
        values: MultiValuesRef<'a>,
        next: usize,
    },
    JsonArray(std::iter::Enumerate<std::slice::Iter<'a, serde_json::Value>>),
    JsonObject(serde_json::map::Iter<'a>),
    StringMap(hash_map::Iter<'a, String, String>),
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = Child<'a>;

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
