// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
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
    Scalar(ValueRef<'a>),
    Collection(MultiValuesRef<'a>),
    Json(&'a serde_json::Value),
    StringMap(&'a HashMap<String, String>),
    Object(&'a BTreeMap<String, NodeId>),
    Text(&'a str),
    Missing(&'a ValueMissing),
}

impl<'a> ReadView<'a> {
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
