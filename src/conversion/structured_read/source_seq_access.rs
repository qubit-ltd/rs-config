// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Collection access retains source indices and never resplits string items.
//!
//! Values and string items are borrowed and yielded lazily; this access does
//! not split an item again. `original_index` remains the first source array
//! index while nested sequence positions are added to the diagnostic path and
//! typed source location. This preserves the original conversion location
//! even when deserialization descends into a nested target.

use qubit_datatype::ConversionSession;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;

use super::deserializer::ReadContext;
use super::deserializer::SourceDeserializer;
use super::node::SourceLocation;
use super::node::SourceSegment;
pub(super) use super::sequence_values::SequenceValues;
use crate::config_deserialize_error::ConfigDeserializeError;

/// Deserializes sequence elements lazily while borrowing values from the
/// prepared input and preserving the outer source index for diagnostics.
pub(super) struct SourceSeqAccess<'tree, 'policy, 'session> {
    /// Borrowed context for resolving nodes and their retained source index.
    context: ReadContext<'tree>,
    /// Borrowed sequence values, yielded one at a time.
    values: SequenceValues<'tree>,
    /// Typed source location extended for each child when one is retained.
    location: Option<SourceLocation>,
    /// Dotted/keyed diagnostic path, extended with each yielded sequence index.
    path: String,
    /// Next position in this sequence, distinct from the retained outer index.
    index: usize,
    /// First source array index used to classify conversion diagnostics.
    original_index: Option<usize>,
    /// Mutable borrow of the read's shared conversion session.
    session: &'session mut ConversionSession<'policy>,
}

impl<'tree, 'policy, 'session> SourceSeqAccess<'tree, 'policy, 'session> {
    /// Creates a lazy sequence accessor borrowing the prepared values and
    /// shared conversion session.
    ///
    /// The outer source index is retained for yielded children; constructing
    /// the accessor does not convert or consume any elements.
    pub(super) fn new(
        context: ReadContext<'tree>,
        values: SequenceValues<'tree>,
        location: Option<SourceLocation>,
        path: String,
        original_index: Option<usize>,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        Self {
            context,
            values,
            location,
            path,
            index: 0,
            original_index,
            session,
        }
    }
}

impl<'de, 'tree, 'policy: 'tree> SeqAccess<'de> for SourceSeqAccess<'tree, 'policy, '_> {
    type Error = ConfigDeserializeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        let Some(value) = self.values.get(self.index) else {
            return Ok(None);
        };
        let path = super::path::child_index(&self.path, self.index);
        let location = self
            .location
            .as_ref()
            .map(|origin| origin.child(SourceSegment::Index(self.index)));
        let original_index = self.original_index.or(Some(self.index));
        self.index += 1;
        seed.deserialize(
            SourceDeserializer::new(self.context, value, location, path.clone(), false, &mut *self.session)
                .with_source_index(original_index),
        )
        .map(Some)
        .map_err(|error| error.with_path(path))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len() - self.index)
    }
}
