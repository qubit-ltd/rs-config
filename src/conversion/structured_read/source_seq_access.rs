// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Collection access retains source indices and never resplits string items.

use qubit_datatype::ConversionSession;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;

use super::deserializer::ReadContext;
use super::deserializer::SourceDeserializer;
use super::node::SourceLocation;
use super::node::SourceSegment;
pub(super) use super::sequence_values::SequenceValues;
use crate::config_deserialize_error::ConfigDeserializeError;

pub(super) struct SourceSeqAccess<'tree, 'policy, 'session> {
    context: ReadContext<'tree>,
    values: SequenceValues<'tree>,
    location: Option<SourceLocation>,
    path: String,
    index: usize,
    original_index: Option<usize>,
    session: &'session mut ConversionSession<'policy>,
}

impl<'tree, 'policy, 'session> SourceSeqAccess<'tree, 'policy, 'session> {
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
