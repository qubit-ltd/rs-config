// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Arena nodes borrow source payloads and own only the necessary path index.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;

use qubit_datatype::DataType;
use qubit_value::MultiValuesRef;
use qubit_value::ValueContainer;
use qubit_value::ValueMissing;
use qubit_value::ValueRef;

pub(super) use super::source_location::SourceLocation;
pub(super) use super::source_segment::SourceSegment;

pub(super) type NodeId = usize;

#[derive(Debug)]
pub(super) enum ReadNode<'a> {
    Scalar(ValueRef<'a>),
    Collection(MultiValuesRef<'a>),
    Json(&'a serde_json::Value),
    StringMap(&'a HashMap<String, String>),
    Object(BTreeMap<String, NodeId>),
    Text(Cow<'a, str>),
    Missing(ValueMissing),
}

impl<'a> ReadNode<'a> {
    /// Natural structural kind for value-free conflict diagnostics.
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Missing(_) | Self::Json(serde_json::Value::Null) => "null",
            Self::Collection(_) | Self::Json(serde_json::Value::Array(_)) => "array",
            Self::Object(_) | Self::StringMap(_) | Self::Json(serde_json::Value::Object(_)) => "object",
            Self::Scalar(ValueRef::Bool(_)) | Self::Json(serde_json::Value::Bool(_)) => "boolean",
            Self::Scalar(
                ValueRef::Int8(_)
                | ValueRef::Int16(_)
                | ValueRef::Int32(_)
                | ValueRef::Int64(_)
                | ValueRef::UInt8(_)
                | ValueRef::UInt16(_)
                | ValueRef::UInt32(_)
                | ValueRef::UInt64(_)
                | ValueRef::Float32(_)
                | ValueRef::Float64(_),
            )
            | Self::Json(serde_json::Value::Number(_)) => "number",
            _ => "string",
        }
    }

    /// Separates source shape without copying any payload.
    pub(super) fn from_container(value: &'a ValueContainer) -> Self {
        match value {
            ValueContainer::Scalar(value) => Self::from_scalar(value.view()),
            ValueContainer::Collection(values) => match values.view() {
                MultiValuesRef::Unset(data_type) => {
                    Self::Missing(ValueMissing::unset_collection(data_type, DataType::Json))
                }
                view => Self::Collection(view),
            },
        }
    }

    pub(super) fn from_scalar(value: ValueRef<'a>) -> Self {
        match value {
            ValueRef::Json(value) => Self::Json(value),
            ValueRef::StringMap(value) => Self::StringMap(value),
            ValueRef::Unset(data_type) => Self::Missing(ValueMissing::unset_scalar(data_type, DataType::Json)),
            view => Self::Scalar(view),
        }
    }
}
