// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde sequence access over configuration values.

use serde::de::{self, SeqAccess};
use serde_json::Value;

use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Sequence access over configuration values.
pub(in crate::config_value_deserializer) struct ConfigSeqAccess<'a> {
    values: std::vec::IntoIter<Value>,
    key: String,
    index: usize,
    options: &'a ReadPolicy,
}

impl<'a> ConfigSeqAccess<'a> {
    /// Creates sequence access.
    pub(in crate::config_value_deserializer) fn new(
        values: Vec<Value>,
        key: String,
        options: &'a ReadPolicy,
    ) -> Self {
        Self {
            values: values.into_iter(),
            key,
            index: 0,
            options,
        }
    }
}

impl<'de> SeqAccess<'de> for ConfigSeqAccess<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next element.
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        let key = format!("{}[{}]", self.key, self.index);
        self.index += 1;
        let error_path = key.clone();
        seed.deserialize(ConfigValueDeserializer::new(value, key, self.options))
            .map_err(|error| error.with_path(error_path))
            .map(Some)
    }
}
