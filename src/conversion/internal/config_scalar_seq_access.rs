// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde sequence access over a delimited scalar string.

use qubit_datatype::ConversionSession;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_value::ValueError;
use serde::de;
use serde::de::SeqAccess;

use crate::ConfigError;
use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Sequence access for scalar strings admitted as collection input.
pub(in crate::config_value_deserializer) struct ConfigScalarSeqAccess<'policy, 'session> {
    values: std::vec::IntoIter<String>,
    key: String,
    index: usize,
    options: &'policy ReadPolicy,
    session: &'session mut ConversionSession<'policy>,
}

impl<'policy, 'session> ConfigScalarSeqAccess<'policy, 'session> {
    /// Creates scalar sequence access from already admitted item text.
    pub(in crate::config_value_deserializer) fn new(
        values: Vec<String>,
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

impl<'de> SeqAccess<'de> for ConfigScalarSeqAccess<'_, '_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next retained item without charging its source again.
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
        self.session.consume_item().map_err(|error| {
            ConfigDeserializeError::from_config(ConfigError::from((
                error_path.as_str(),
                ValueError::from(DataConversionError::limit_exceeded(
                    DataType::String,
                    DataType::String,
                    error,
                )),
            )))
        })?;
        seed.deserialize(ConfigValueDeserializer::new_precharged(
            serde_json::Value::String(value),
            key,
            self.options,
            &mut *self.session,
        ))
        .map_err(|error| error.with_path(error_path))
        .map(Some)
    }
}

/// Admits and splits a scalar source once for sequence deserialization.
pub(in crate::config_value_deserializer) fn admit_scalar_items(
    key: &str,
    value: &str,
    options: &ReadPolicy,
    session: &mut ConversionSession<'_>,
) -> Result<Vec<String>, ConfigDeserializeError> {
    let bytes = u64::try_from(value.len()).unwrap();
    session
        .check_collection_source_bytes(bytes)
        .map_err(|error| {
            ConfigDeserializeError::from_config(ConfigError::from((
                key,
                ValueError::from(DataConversionError::limit_exceeded(
                    DataType::String,
                    DataType::String,
                    error,
                )),
            )))
        })?;
    session.consume_input_bytes(bytes).map_err(|error| {
        ConfigDeserializeError::from_config(ConfigError::from((
            key,
            ValueError::from(DataConversionError::limit_exceeded(
                DataType::String,
                DataType::String,
                error,
            )),
        )))
    })?;

    options
        .conversion_policy()
        .collection()
        .scalar_items(options.conversion_limits().collection(), value)
        .map(|item| {
            item.map(|item| item.value.to_owned()).map_err(|error| {
                ConfigDeserializeError::from_config(ConfigError::from((
                    key,
                    ValueError::from(error.into_data_conversion_error(DataType::String)),
                )))
            })
        })
        .collect()
}
