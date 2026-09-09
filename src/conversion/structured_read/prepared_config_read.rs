// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Selection and bounded construction of the logical configuration path tree.

use std::collections::BTreeMap;

use qubit_budget::MeasuredBudgetError;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceQuantity;
use qubit_budget::json::JsonMeasurement;
use qubit_budget::json::JsonValueBudget;
use qubit_budget::json::JsonValueLimits;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionResource;
use qubit_datatype::DataType;
use qubit_value::ValueError;

use super::node::NodeId;
use super::node::ReadNode;
use super::node::SourceLocation;
use super::node::SourceSegment;
use super::source_index::SourceIndex;
use crate::ConfigError;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::Property;

#[derive(Debug)]
pub(super) struct PreparedConfigRead<'a> {
    pub(super) overlays: BTreeMap<SourceLocation, String>,
    pub(super) nodes: Vec<ReadNode<'a>>,
    pub(super) sources: SourceIndex<'a>,
    pub(super) root: NodeId,
    pub(super) root_path: String,
}

/// Bounds the path index before allocating nodes or copying keys. Full input
/// accounting is a separate traversal of the completed logical tree.
struct IndexCapacity {
    budget: JsonValueBudget<ConversionResource, u64>,
    input_keys: ResourceBudget<ConversionResource, u64>,
    payload_keys: ResourceBudget<ConversionResource, u64>,
    limits: ConversionLimits,
}

impl IndexCapacity {
    fn new(limits: &ConversionLimits) -> Self {
        Self {
            budget: JsonValueBudget::new(
                JsonValueLimits::builder()
                    .structure_limits(
                        limits
                            .structured()
                            .value()
                            .structure_limits()
                            .to_builder()
                            .nodes_limit(*limits.operation().structured_nodes_limit())
                            .build(),
                    )
                    .build(),
            ),
            input_keys: ResourceBudget::from_limit(*limits.operation().input_bytes_limit()),
            payload_keys: ResourceBudget::from_limit(*limits.operation().structured_payload_bytes_limit()),
            limits: limits.clone(),
        }
    }

    fn admit(&mut self, measurement: JsonMeasurement, path: &str) -> ConfigResult<()> {
        let mut transaction = self.budget.transaction();
        transaction
            .try_admit(measurement)
            .and_then(|()| transaction.commit())
            .map_err(|source| {
                ConfigError::from((
                    path,
                    ValueError::JsonProjectionLimit {
                        data_type: DataType::Json,
                        source_index: None,
                        source,
                    },
                ))
            })
    }

    fn key(&mut self, bytes: usize, path: &str) -> ConfigResult<()> {
        self.input_keys
            .try_consume_usize(bytes)
            .map_err(|error| index_error(path, error))?;
        self.payload_keys
            .try_consume_usize(bytes)
            .map_err(|error| index_error(path, error))?;
        self.admit(JsonMeasurement::Key { bytes }, path)
    }

    fn entries(&self, count: usize, path: &str) -> ConfigResult<()> {
        let count = u64::try_from_usize(count).map_err(|error| {
            index_error(
                path,
                MeasuredBudgetError::quantity(ConversionResource::MapEntries, error),
            )
        })?;
        self.limits
            .structured()
            .max_map_entries_limit()
            .check(count)
            .map_err(|error| index_error(path, error.into()))
    }
}

fn index_error(path: &str, source: MeasuredBudgetError<ConversionResource, u64>) -> ConfigError {
    ConfigError::from((
        path,
        ValueError::JsonProjectionLimit {
            data_type: DataType::Json,
            source_index: None,
            source,
        },
    ))
}

impl<'a> PreparedConfigRead<'a> {
    fn push(
        &mut self,
        node: ReadNode<'a>,
        origin: Option<SourceLocation>,
        depth: usize,
        capacity: &mut IndexCapacity,
        path: &str,
    ) -> ConfigResult<NodeId> {
        capacity.admit(JsonMeasurement::Null { depth }, path)?;
        let id = self.nodes.len();
        self.nodes.push(node);
        self.sources.push_origin(origin);
        Ok(id)
    }

    /// Expands only objects needed to merge dotted paths; untouched payloads
    /// remain borrowed JSON or map nodes.
    fn expand_object(
        &mut self,
        id: NodeId,
        depth: usize,
        capacity: &mut IndexCapacity,
        path: &str,
    ) -> ConfigResult<()> {
        let origin = self.sources.origin(id).cloned();
        let mut children = BTreeMap::new();
        let source = match &self.nodes[id] {
            ReadNode::Object(_) => return Ok(()),
            ReadNode::Json(value) => ReadNode::Json(value),
            ReadNode::StringMap(value) => ReadNode::StringMap(value),
            value => return Err(conflict(path, value.kind(), "object")),
        };
        match source {
            ReadNode::Json(serde_json::Value::Object(values)) => {
                capacity.entries(values.len(), path)?;
                for (key, value) in values {
                    capacity.key(key.len(), path)?;
                    let location = origin
                        .as_ref()
                        .map(|origin| origin.child(SourceSegment::Key(key.clone())));
                    let child = self.push(ReadNode::Json(value), location, depth.saturating_add(1), capacity, path)?;
                    children.insert(key.clone(), child);
                }
            }
            ReadNode::StringMap(values) => {
                capacity.entries(values.len(), path)?;
                for (key, value) in values {
                    capacity.key(key.len(), path)?;
                    let location = origin
                        .as_ref()
                        .map(|origin| origin.child(SourceSegment::Key(key.clone())));
                    let child = self.push(
                        ReadNode::Text(std::borrow::Cow::Borrowed(value)),
                        location,
                        depth.saturating_add(1),
                        capacity,
                        path,
                    )?;
                    children.insert(key.clone(), child);
                }
            }
            value => return Err(conflict(path, value.kind(), "object")),
        }
        self.nodes[id] = ReadNode::Object(children);
        Ok(())
    }

    fn insert_property(&mut self, key: &str, property: &'a Property, capacity: &mut IndexCapacity) -> ConfigResult<()> {
        let mut current = self.root;
        let mut depth = 1usize;
        let mut path_end = self.root_path.len();
        let mut segments = key.split('.').peekable();
        while let Some(segment) = segments.next() {
            self.expand_object(current, depth, capacity, &property.name()[..path_end])?;
            if path_end != 0 {
                path_end += 1;
            }
            path_end += segment.len();
            depth = depth.saturating_add(1);
            let existing = match &self.nodes[current] {
                ReadNode::Object(children) => children.get(segment).copied(),
                _ => unreachable!("expanded object"),
            };
            if existing.is_none() {
                let ReadNode::Object(children) = &self.nodes[current] else {
                    unreachable!()
                };
                capacity.entries(children.len().saturating_add(1), property.name())?;
                capacity.key(segment.len(), property.name())?;
            }
            if segments.peek().is_none() {
                let location = SourceLocation {
                    property_index: self.sources.len(),
                    segments: Vec::new(),
                };
                let incoming = ReadNode::from_container(property.value());
                if let Some(existing) = existing {
                    self.merge_object(
                        existing,
                        incoming,
                        location,
                        depth,
                        capacity,
                        &property.name()[..path_end],
                    )?;
                } else {
                    let child = self.push(incoming, Some(location), depth, capacity, property.name())?;
                    let ReadNode::Object(children) = &mut self.nodes[current] else {
                        unreachable!()
                    };
                    children.insert(segment.to_owned(), child);
                }
                self.sources.push_property(property);
                return Ok(());
            }
            current = match existing {
                Some(existing) => existing,
                None => {
                    let child = self.push(
                        ReadNode::Object(BTreeMap::new()),
                        None,
                        depth,
                        capacity,
                        property.name(),
                    )?;
                    let ReadNode::Object(children) = &mut self.nodes[current] else {
                        unreachable!()
                    };
                    children.insert(segment.to_owned(), child);
                    child
                }
            };
        }
        Ok(())
    }

    /// Iterative object merging rejects duplicate leaves rather than replacing
    /// whichever property happened to be visited first.
    fn merge_object(
        &mut self,
        id: NodeId,
        incoming: ReadNode<'a>,
        origin: SourceLocation,
        depth: usize,
        capacity: &mut IndexCapacity,
        path: &str,
    ) -> ConfigResult<()> {
        let mut pending = vec![(id, incoming, origin, depth, path.to_owned())];
        while let Some((id, incoming, origin, depth, path)) = pending.pop() {
            if self.nodes[id].kind() != "object" || incoming.kind() != "object" {
                return Err(conflict(&path, self.nodes[id].kind(), incoming.kind()));
            }
            self.expand_object(id, depth, capacity, &path)?;
            let entries: Box<dyn Iterator<Item = (&'a str, ReadNode<'a>)> + 'a> = match incoming {
                ReadNode::Json(serde_json::Value::Object(values)) => {
                    Box::new(values.iter().map(|(key, value)| (key.as_str(), ReadNode::Json(value))))
                }
                ReadNode::StringMap(values) => Box::new(
                    values
                        .iter()
                        .map(|(key, value)| (key.as_str(), ReadNode::Text(std::borrow::Cow::Borrowed(value)))),
                ),
                value => return Err(conflict(&path, "object", value.kind())),
            };
            for (key, node) in entries {
                let ReadNode::Object(children) = &self.nodes[id] else {
                    unreachable!()
                };
                if !children.contains_key(key) {
                    capacity.entries(children.len().saturating_add(1), &path)?;
                    capacity.key(key.len(), &path)?;
                }
                let child_origin = origin.child(SourceSegment::Key(key.to_owned()));
                if let Some(&child) = children.get(key) {
                    pending.push((
                        child,
                        node,
                        child_origin,
                        depth.saturating_add(1),
                        super::path::child_key(&path, key),
                    ));
                } else {
                    let child = self.push(node, Some(child_origin), depth.saturating_add(1), capacity, &path)?;
                    let ReadNode::Object(children) = &mut self.nodes[id] else {
                        unreachable!()
                    };
                    children.insert(key.to_owned(), child);
                }
            }
        }
        Ok(())
    }
}

fn conflict(path: &str, existing: &str, incoming: &str) -> ConfigError {
    ConfigError::KeyConflict {
        source_id: None,
        path: path.to_owned(),
        existing: existing.to_owned(),
        incoming: incoming.to_owned(),
    }
}

pub(super) fn prepare_read<'a, R: ConfigReader + ?Sized>(
    reader: &'a R,
    prefix: &str,
    interpolate: bool,
) -> ConfigResult<PreparedConfigRead<'a>> {
    let root_path = reader.resolve_key(prefix)?;
    let mut prepared = PreparedConfigRead {
        overlays: BTreeMap::new(),
        nodes: Vec::new(),
        sources: SourceIndex::tree(),
        root: 0,
        root_path,
    };
    let mut capacity = IndexCapacity::new(reader.read_policy().conversion_limits());
    if !prefix.is_empty()
        && let Some(property) = reader.get_property(prefix)?
    {
        if reader.contains_section(prefix)? {
            return Err(conflict(&prepared.root_path, "exact value", "nested child keys"));
        }
        capacity.admit(JsonMeasurement::Null { depth: 1 }, property.name())?;
        prepared.nodes = vec![ReadNode::from_container(property.value())];
        prepared.sources = SourceIndex::exact(property);
    } else {
        let path = prepared.root_path.clone();
        prepared.push(ReadNode::Object(BTreeMap::new()), None, 1, &mut capacity, &path)?;
        for (key, property) in reader.iter() {
            let relative = if prefix.is_empty() {
                Some(key)
            } else {
                key.strip_prefix(prefix).and_then(|key| key.strip_prefix('.'))
            };
            if let Some(relative) = relative {
                prepared.insert_property(relative, property, &mut capacity)?;
            }
        }
    }
    super::input_budget::admit_input(&prepared, reader.read_policy())?;
    let exact = prepared.sources.is_exact();
    if interpolate {
        prepared.overlays = if exact {
            super::interpolation_overlay::interpolate(&prepared, reader)?
        } else {
            super::interpolation_overlay::interpolate(&prepared, &reader.section(prefix)?)?
        };
        super::input_budget::admit_input(&prepared, reader.read_policy())?;
    }
    super::interpolation_overlay::classify_missing(&mut prepared, reader.read_policy(), exact)?;
    Ok(prepared)
}
