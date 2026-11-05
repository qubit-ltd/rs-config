// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed view over prepared nodes and native source values.
//!
//! Views retain the source lifetime instead of materializing another value
//! tree. Their borrowed data is valid only while the source configuration and
//! prepared projection remain alive; owned text is used only for overlays.

use std::collections::BTreeMap;
use std::collections::HashMap;

use qubit_value::MultiValuesRef;
use qubit_value::ValueMissing;
use qubit_value::ValueRef;

use super::node::NodeId;
use super::node::ReadNode;

/// Borrowed view of a prepared node or a supported source value.
#[must_use]
#[derive(Clone, Copy)]
pub(super) enum ReadView<'a> {
    /// Native scalar value.
    Scalar(
        /// Borrowed or copied primitive value from the source.
        ValueRef<'a>,
    ),
    /// Native homogeneous collection.
    Collection(
        /// Borrowed collection payload supplied by the source.
        MultiValuesRef<'a>,
    ),
    /// Borrowed JSON value.
    Json(
        /// JSON value retained from the original input.
        &'a serde_json::Value,
    ),
    /// Borrowed string-keyed map.
    StringMap(
        /// String map retained from the original input.
        &'a HashMap<String, String>,
    ),
    /// Indexed object children.
    Object(
        /// Sorted child-name to prepared-node index mapping.
        &'a BTreeMap<String, NodeId>,
    ),
    /// Text leaf.
    Text(
        /// Borrowed source or overlay text.
        &'a str,
    ),
    /// Missing value metadata.
    Missing(
        /// Original type metadata for a missing source value.
        &'a ValueMissing,
    ),
}

impl<'a> ReadView<'a> {
    /// Creates a borrowed view over one prepared node.
    ///
    /// # Parameters
    ///
    /// * `node` - Prepared arena node whose borrowed representation is exposed.
    #[inline]
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
    ///
    /// # Parameters
    ///
    /// * `value` - Scalar source, including JSON and string-map variants.
    #[inline]
    pub(super) fn from_scalar(value: ValueRef<'a>) -> Self {
        match value {
            ValueRef::Json(value) => Self::Json(value),
            ValueRef::StringMap(value) => Self::StringMap(value),
            value => Self::Scalar(value),
        }
    }

    /// Only actual string leaves participate in placeholder substitution.
    #[inline]
    pub(super) fn string(self) -> Option<&'a str> {
        match self {
            Self::Scalar(ValueRef::String(value)) | Self::Text(value) => Some(value),
            Self::Json(serde_json::Value::String(value)) => Some(value),
            _ => None,
        }
    }
}
