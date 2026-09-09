// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Iterative borrowed traversal with source indices independent of paths.

use std::borrow::Cow;

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
    children: Box<dyn Iterator<Item = Child<'a>> + 'a>,
    path: Cow<'a, str>,
    location: Option<SourceLocation>,
    depth: usize,
    original_index: Option<usize>,
}

fn children<'a>(
    value: ReadView<'a>,
    prepared: &'a PreparedConfigRead<'_>,
) -> Option<Box<dyn Iterator<Item = Child<'a>> + 'a>> {
    match value {
        ReadView::Object(values) => Some(Box::new(values.iter().map(|(key, id)| Child {
            segment: Segment::Key(key),
            value: ReadView::from_node(&prepared.nodes[*id]),
            origin: prepared.sources.origin(*id),
        }))),
        ReadView::Collection(values) => Some(Box::new((0..values.len()).map(move |index| Child {
            segment: Segment::Index(index),
            value: ReadView::from_scalar(values.get(index).expect("source index in range")),
            origin: None,
        }))),
        ReadView::Json(serde_json::Value::Array(values)) => {
            Some(Box::new(values.iter().enumerate().map(|(index, value)| Child {
                segment: Segment::Index(index),
                value: ReadView::Json(value),
                origin: None,
            })))
        }
        ReadView::Json(serde_json::Value::Object(values)) => Some(Box::new(values.iter().map(|(key, value)| Child {
            segment: Segment::Key(key),
            value: ReadView::Json(value),
            origin: None,
        }))),
        ReadView::StringMap(values) => Some(Box::new(values.iter().map(|(key, value)| Child {
            segment: Segment::Key(key),
            value: ReadView::Text(value),
            origin: None,
        }))),
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
