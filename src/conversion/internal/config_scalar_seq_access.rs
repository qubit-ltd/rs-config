// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde sequence access over a delimited scalar string.
// qubit-style: allow source-test-pair

use qubit_datatype::AdmittedScalarSource;
use qubit_datatype::ConversionSession;
use qubit_value::ValueError;
use serde::de;
use serde::de::SeqAccess;

use crate::ConfigError;
use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Sequence access borrowing a charged scalar source without copying its items.
pub(in crate::config_value_deserializer) struct ConfigScalarSeqAccess<'policy, 'session, 'source> {
    /// Source-bound admission owns the exclusive session borrow.
    source: AdmittedScalarSource<'session, 'policy, 'source>,
    /// Parent configuration path used for source-index diagnostics.
    key: String,
    /// Read semantics forwarded to each nested deserializer.
    options: &'policy ReadPolicy,
}

impl<'policy: 'source, 'session, 'source> ConfigScalarSeqAccess<'policy, 'session, 'source> {
    /// Admits the entire source before exposing its lazily split elements.
    ///
    /// Returns a keyed conversion error if source bytes exceed the configured
    /// collection or cumulative input budget. No item text is copied.
    pub(in crate::config_value_deserializer) fn new(
        value: &'source str,
        key: String,
        options: &'policy ReadPolicy,
        session: &'session mut ConversionSession<'policy>,
    ) -> Result<Self, ConfigDeserializeError> {
        let source = session.admit_scalar_string_source(value).map_err(|error| {
            ConfigDeserializeError::from_config(ConfigError::from((key.as_str(), ValueError::from(error))))
        })?;
        Ok(Self { source, key, options })
    }
}

impl<'de, 'policy: 'source, 'source> SeqAccess<'de> for ConfigScalarSeqAccess<'policy, '_, 'source> {
    type Error = ConfigDeserializeError;

    /// Deserializes one original source position after its item budget admits
    /// it.
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let Some(item) = self.source.next_item() else {
            return Ok(None);
        };
        let admitted = item.map_err(|error| {
            let (source_index, error) = error.into_parts();
            let key = format!("{}[{}]", self.key, source_index);
            ConfigDeserializeError::from_config(ConfigError::from((key.as_str(), ValueError::from(error))))
        })?;
        let key = format!("{}[{}]", self.key, admitted.source_index());
        let error_path = key.clone();
        seed.deserialize(ConfigValueDeserializer::new_admitted(key, self.options, admitted))
            .map_err(|error| error.with_path(error_path))
            .map(Some)
    }
}
