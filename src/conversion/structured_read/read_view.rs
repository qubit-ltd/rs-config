// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed view over prepared nodes and native source values.

use std::collections::BTreeMap;
use std::collections::HashMap;

use qubit_value::MultiValuesRef;
use qubit_value::ValueMissing;
use qubit_value::ValueRef;

use super::node::NodeId;
use super::node::ReadNode;

#[derive(Clone, Copy)]
pub(super) enum ReadView<'a> {
    /// Native scalar value.
    Scalar(ValueRef<'a>),
    /// Native homogeneous collection.
    Collection(MultiValuesRef<'a>),
    /// Borrowed JSON value.
    Json(&'a serde_json::Value),
    /// Borrowed string-keyed map.
    StringMap(&'a HashMap<String, String>),
    /// Indexed object children.
    Object(&'a BTreeMap<String, NodeId>),
    /// Text leaf.
    Text(&'a str),
    /// Missing value metadata.
    Missing(&'a ValueMissing),
}

impl<'a> ReadView<'a> {
    /// Creates a borrowed view over one prepared node.
    pub(super) fn from_node(node: &'a ReadNode<'_>) -> Self {
        match node {
            ReadNode::Scalar(value) => Self::from_scalar(*value),
            ReadNode::Collection(value) => Self::Collection(*value),
            ReadNode::Json(value) => Self::Json(value),
            ReadNode::StringMap(value) => Self::StringMap(value),
            ReadNode::Object(value) => Self::Object(value),
            ReadNode::Text(value) => Self::Text(value),
            ReadNode::Missing(value) => Self::Missing(value),
        }
    }

    /// Creates a view over one native scalar.
    pub(super) fn from_scalar(value: ValueRef<'a>) -> Self {
        match value {
            ValueRef::Json(value) => Self::Json(value),
            ValueRef::StringMap(value) => Self::StringMap(value),
            value => Self::Scalar(value),
        }
    }

    /// Only actual string leaves participate in placeholder substitution.
    pub(super) fn string(self) -> Option<&'a str> {
        match self {
            Self::Scalar(ValueRef::String(value)) | Self::Text(value) => Some(value),
            Self::Json(serde_json::Value::String(value)) => Some(value),
            _ => None,
        }
    }
}
