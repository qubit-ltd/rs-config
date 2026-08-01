// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Serde map access over configuration objects.

use serde::de::{
    self,
    IntoDeserializer,
    MapAccess,
    value::StringDeserializer,
};
use serde_json::{
    Map,
    Value,
};

use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Map access over configuration objects.
pub(in crate::config_value_deserializer) struct ConfigMapAccess<'a> {
    entries: std::vec::IntoIter<(String, Value)>,
    next_value: Option<(String, Value)>,
    key: String,
    options: &'a ReadPolicy,
}

impl<'a> ConfigMapAccess<'a> {
    /// Creates map access.
    pub(in crate::config_value_deserializer) fn new(
        values: Map<String, Value>,
        key: String,
        options: &'a ReadPolicy,
    ) -> Self {
        Self {
            entries: values.into_iter().collect::<Vec<_>>().into_iter(),
            next_value: None,
            key,
            options,
        }
    }
}

impl<'de> MapAccess<'de> for ConfigMapAccess<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next key.
    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.entries.next() else {
            return Ok(None);
        };
        let key_deserializer: StringDeserializer<Self::Error> =
            key.clone().into_deserializer();
        self.next_value = Some((key, value));
        seed.deserialize(key_deserializer).map(Some)
    }

    /// Deserializes the value for the last key.
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let (key, value) = self
            .next_value
            .take()
            .expect("map value requested before key");
        let child_key = if self.key.is_empty() {
            key
        } else {
            format!("{}.{}", self.key, key)
        };
        let error_path = child_key.clone();
        seed.deserialize(ConfigValueDeserializer::new(
            value,
            child_key,
            self.options,
        ))
        .map_err(|error| error.with_path(error_path))
    }
}
