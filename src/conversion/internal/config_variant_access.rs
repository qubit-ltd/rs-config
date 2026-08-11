// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Serde variant access over a configuration enum payload.

use qubit_datatype::ConversionSession;
use serde::Deserialize;
use serde::de;
use serde::de::DeserializeSeed;
use serde::de::VariantAccess;
use serde::de::Visitor;
use serde_json::Value;

use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;

/// Variant access over a configuration enum payload.
pub(in crate::config_value_deserializer) struct ConfigVariantAccess<'policy, 'session> {
    value: Option<Value>,
    key: String,
    options: &'policy ReadPolicy,
    session: &'session mut ConversionSession<'policy>,
}

impl<'policy, 'session> ConfigVariantAccess<'policy, 'session> {
    /// Creates access for an optional enum payload.
    pub(in crate::config_value_deserializer) fn new(
        value: Option<Value>,
        key: String,
        options: &'policy ReadPolicy,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        Self {
            value,
            key,
            options,
            session,
        }
    }
}

impl<'de> VariantAccess<'de> for ConfigVariantAccess<'_, '_> {
    type Error = ConfigDeserializeError;

    /// Deserializes a unit variant.
    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None | Some(Value::Null) => Ok(()),
            Some(value) => Deserialize::deserialize(ConfigValueDeserializer::new(
                value,
                self.key,
                self.options,
                self.session,
            )),
        }
    }

    /// Deserializes a newtype variant payload.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let value = self.value.ok_or_else(|| {
            de::Error::invalid_type(de::Unexpected::UnitVariant, &"newtype variant payload")
        })?;
        seed.deserialize(ConfigValueDeserializer::new(
            value,
            self.key,
            self.options,
            self.session,
        ))
    }

    /// Deserializes a tuple variant payload.
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value.ok_or_else(|| {
            de::Error::invalid_type(de::Unexpected::UnitVariant, &"tuple variant payload")
        })?;
        de::Deserializer::deserialize_tuple(
            ConfigValueDeserializer::new(value, self.key, self.options, self.session),
            len,
            visitor,
        )
    }

    /// Deserializes a struct variant payload.
    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value.ok_or_else(|| {
            de::Error::invalid_type(de::Unexpected::UnitVariant, &"struct variant payload")
        })?;
        de::Deserializer::deserialize_struct(
            ConfigValueDeserializer::new(value, self.key, self.options, self.session),
            "",
            fields,
            visitor,
        )
    }
}
