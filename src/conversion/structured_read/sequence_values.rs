// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Native homogeneous or JSON sequence input.

use qubit_value::MultiValuesRef;

use super::traversal::ReadView;

pub(super) enum SequenceValues<'tree> {
    Collection(MultiValuesRef<'tree>),
    Json(&'tree [serde_json::Value]),
}

impl<'tree> SequenceValues<'tree> {
    pub(super) fn from_view(value: ReadView<'tree>) -> Option<Self> {
        match value {
            ReadView::Collection(values) => Some(Self::Collection(values)),
            ReadView::Json(serde_json::Value::Array(values)) => Some(Self::Json(values)),
            _ => None,
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Collection(values) => values.len(),
            Self::Json(values) => values.len(),
        }
    }
    pub(super) fn get(&self, index: usize) -> Option<ReadView<'tree>> {
        match self {
            Self::Collection(values) => values.get(index).map(ReadView::from_scalar),
            Self::Json(values) => values.get(index).map(ReadView::Json),
        }
    }
}
