// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Map entries remain native borrowed iterators until visited.

use qubit_datatype::ConversionSession;
use serde::de::DeserializeSeed;
use serde::de::MapAccess;
use serde::de::value::StrDeserializer;

use super::deserializer::ReadContext;
use super::deserializer::SourceDeserializer;
use super::deserializer::type_error;
pub(super) use super::map_entries::MapEntries;
use super::node::SourceLocation;
use super::node::SourceSegment;
use super::traversal::ReadView;
use crate::config_deserialize_error::ConfigDeserializeError;

pub(super) struct SourceMapAccess<'tree, 'policy, 'session> {
    context: ReadContext<'tree>,
    entries: MapEntries<'tree>,
    location: Option<SourceLocation>,
    path: String,
    original_index: Option<usize>,
    session: &'session mut ConversionSession<'policy>,
    pending: Option<(&'tree str, ReadView<'tree>, Option<SourceLocation>)>,
}

impl<'tree, 'policy, 'session> SourceMapAccess<'tree, 'policy, 'session> {
    pub(super) fn new(
        context: ReadContext<'tree>,
        entries: MapEntries<'tree>,
        location: Option<SourceLocation>,
        path: String,
        original_index: Option<usize>,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        Self {
            context,
            entries,
            location,
            path,
            original_index,
            session,
            pending: None,
        }
    }
}

impl<'de, 'tree, 'policy: 'tree> MapAccess<'de> for SourceMapAccess<'tree, 'policy, '_> {
    type Error = ConfigDeserializeError;
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
        if self.pending.is_some() {
            return Err(type_error(&self.path, "a value for the previous key"));
        }
        let Some((key, value, origin)) = self.entries.next(self.context) else {
            return Ok(None);
        };
        self.pending = Some((key, value, origin));
        seed.deserialize(StrDeserializer::<ConfigDeserializeError>::new(key))
            .map(Some)
            .map_err(|error| error.with_path(self.path.clone()))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
        let (key, value, origin) = self
            .pending
            .take()
            .ok_or_else(|| type_error(&self.path, "a preceding map key"))?;
        let path = super::path::child_key(&self.path, key);
        let location = origin.or_else(|| {
            self.location
                .as_ref()
                .map(|origin| origin.child(SourceSegment::Key(key.to_owned())))
        });
        seed.deserialize(
            SourceDeserializer::new(self.context, value, location, path.clone(), true, &mut *self.session)
                .with_source_index(self.original_index),
        )
        .map_err(|error| error.with_path(path))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}
