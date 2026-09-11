// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Iterative borrowed traversal with source indices independent of paths.

use std::borrow::Cow;
use std::collections::btree_map;
use std::collections::hash_map;

use qubit_value::MultiValuesRef;

use super::node::SourceLocation;
use super::node::SourceSegment;
use super::prepared_read::PreparedConfigRead;
pub(super) use super::read_view::ReadView;
pub(super) use super::visit::Visit;
use crate::ConfigResult;

enum Segment<'a> {
    Key(&'a str),
    Index(usize),
}
struct Child<'a> {
    segment: Segment<'a>,
    value: ReadView<'a>,
    origin: Option<&'a SourceLocation>,
}

struct Frame<'a> {
    children: ChildIter<'a>,
    path: Cow<'a, str>,
    location: Option<SourceLocation>,
    depth: usize,
    original_index: Option<usize>,
}

enum ChildIter<'a> {
    Object {
        values: btree_map::Iter<'a, String, super::node::NodeId>,
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

fn children<'a>(value: ReadView<'a>, prepared: &'a PreparedConfigRead<'a>) -> Option<ChildIter<'a>> {
    match value {
        ReadView::Object(values) => Some(ChildIter::Object {
            values: values.iter(),
            prepared,
        }),
        ReadView::Collection(values) => Some(ChildIter::Collection { values, next: 0 }),
        ReadView::Json(serde_json::Value::Array(values)) => Some(ChildIter::JsonArray(values.iter().enumerate())),
        ReadView::Json(serde_json::Value::Object(values)) => Some(ChildIter::JsonObject(values.iter())),
        ReadView::StringMap(values) => Some(ChildIter::StringMap(values.iter())),
        _ => None,
    }
}

/// Invokes admission before creating the next iterator frame. Only ancestor
/// frames remain live, so a large array cannot allocate a queue of its leaves.
pub(super) fn walk<'a>(
    prepared: &'a PreparedConfigRead<'_>,
    track_locations: bool,
    mut visit: impl FnMut(&Visit<'a>) -> ConfigResult<()>,
) -> ConfigResult<()> {
    let mut next = Some(Visit {
        value: ReadView::from_node(&prepared.nodes[prepared.root]),
        location: if track_locations {
            prepared.sources.origin(prepared.root).cloned()
        } else {
            None
        },
        path: Cow::Borrowed(&prepared.root_path),
        depth: 1,
        key: None,
        original_index: None,
    });
    let mut stack = Vec::<Frame<'a>>::new();
    while let Some(current) = next.take() {
        visit(&current)?;
        if let Some(children) = children(current.value, prepared) {
            stack.push(Frame {
                children,
                path: current.path,
                location: current.location,
                depth: current.depth.saturating_add(1),
                original_index: current.original_index,
            });
        }
        while let Some(frame) = stack.last_mut() {
            if let Some(child) = frame.children.next() {
                let (key, path, segment, original_index) = match child.segment {
                    Segment::Key(key) => (
                        Some(key),
                        super::path::child_key(&frame.path, key),
                        track_locations.then(|| SourceSegment::Key(key.to_owned())),
                        frame.original_index,
                    ),
                    Segment::Index(index) => (
                        None,
                        super::path::child_index(&frame.path, index),
                        track_locations.then_some(SourceSegment::Index(index)),
                        frame.original_index.or(Some(index)),
                    ),
                };
                let location = if track_locations {
                    child.origin.cloned().or_else(|| {
                        frame
                            .location
                            .as_ref()
                            .map(|origin| origin.child(segment.expect("tracked segment")))
                    })
                } else {
                    None
                };
                next = Some(Visit {
                    value: child.value,
                    location,
                    path: Cow::Owned(path),
                    depth: frame.depth,
                    key,
                    original_index,
                });
                break;
            }
            stack.pop();
        }
    }
    Ok(())
}
