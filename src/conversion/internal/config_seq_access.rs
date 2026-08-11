// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Serde sequence access over configuration values.

use qubit_datatype::ConversionSession;
use serde::de;
use serde::de::SeqAccess;
use serde_json::Value;

use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Sequence access over configuration values.
pub(in crate::config_value_deserializer) struct ConfigSeqAccess<
    'policy,
    'session,
> {
    values: std::vec::IntoIter<Value>,
    key: String,
    index: usize,
    options: &'policy ReadPolicy,
    session: &'session mut ConversionSession<'policy>,
}

impl<'policy, 'session> ConfigSeqAccess<'policy, 'session> {
    /// Creates sequence access.
    pub(in crate::config_value_deserializer) fn new(
        values: Vec<Value>,
        key: String,
        options: &'policy ReadPolicy,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        Self {
            values: values.into_iter(),
            key,
            index: 0,
            options,
            session,
        }
    }
}

impl<'de> SeqAccess<'de> for ConfigSeqAccess<'_, '_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next element.
    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        let key = format!("{}[{}]", self.key, self.index);
        self.index += 1;
        let error_path = key.clone();
        seed.deserialize(ConfigValueDeserializer::new(
            value,
            key,
            self.options,
            &mut *self.session,
        ))
        .map_err(|error| error.with_path(error_path))
        .map(Some)
    }
}
