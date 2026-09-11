// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Source-bound scalar-list admission is reused for each Serde item.

use qubit_datatype::AdmittedScalarSource;
use qubit_datatype::ConversionSession;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;

use super::deserializer::ReadContext;
use super::deserializer::SourceDeserializer;
use super::deserializer::conversion_error;
use crate::config_deserialize_error::ConfigDeserializeError;

pub(super) struct ScalarSeqAccess<'tree, 'policy, 'session> {
    context: ReadContext<'tree>,
    source: AdmittedScalarSource<'session, 'policy, 'tree>,
    path: String,
}

impl<'tree, 'policy: 'tree, 'session> ScalarSeqAccess<'tree, 'policy, 'session> {
    pub(super) fn new(
        context: ReadContext<'tree>,
        text: &'tree str,
        path: String,
        session: &'session mut ConversionSession<'policy>,
    ) -> Result<Self, ConfigDeserializeError> {
        let source = session
            .admit_scalar_string_source(text)
            .map_err(|error| conversion_error(&path, None, error))?;
        Ok(Self { context, source, path })
    }

    pub(super) fn has_remaining(&mut self) -> Result<bool, ConfigDeserializeError> {
        match self.source.next_item() {
            None => Ok(false),
            Some(Ok(_)) => Ok(true),
            Some(Err(error)) => {
                let (index, error) = error.into_parts();
                Err(conversion_error(
                    &super::path::child_index(&self.path, index),
                    Some(index),
                    error,
                ))
            }
        }
    }
}

impl<'de, 'tree, 'policy: 'tree> SeqAccess<'de> for ScalarSeqAccess<'tree, 'policy, '_> {
    type Error = ConfigDeserializeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        let Some(item) = self.source.next_item() else {
            return Ok(None);
        };
        let item = item.map_err(|error| {
            let (index, error) = error.into_parts();
            conversion_error(&super::path::child_index(&self.path, index), Some(index), error)
        })?;
        let path = super::path::child_index(&self.path, item.source_index());
        seed.deserialize(SourceDeserializer::admitted(self.context, path.clone(), item))
            .map(Some)
            .map_err(|error| error.with_path(path))
    }
}
