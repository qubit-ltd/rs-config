// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde variant access over a configuration enum payload.

use serde::de::{
    self,
    DeserializeSeed,
    VariantAccess,
    Visitor,
};
use serde_json::Value;

use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ConfigReadOptions;

/// Variant access over a configuration enum payload.
pub(in crate::config_value_deserializer) struct ConfigVariantAccess<'a> {
    value: Option<Value>,
    key: String,
    options: &'a ConfigReadOptions,
}

impl<'a> ConfigVariantAccess<'a> {
    /// Creates access for an optional enum payload.
    pub(in crate::config_value_deserializer) fn new(
        value: Option<Value>,
        key: String,
        options: &'a ConfigReadOptions,
    ) -> Self {
        Self {
            value,
            key,
            options,
        }
    }
}

impl<'de> VariantAccess<'de> for ConfigVariantAccess<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes a unit variant.
    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None | Some(Value::Null) => Ok(()),
            Some(value) => serde::Deserialize::deserialize(
                ConfigValueDeserializer::new(value, self.key, self.options),
            ),
        }
    }

    /// Deserializes a newtype variant payload.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let value = self.value.ok_or_else(|| {
            de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant payload",
            )
        })?;
        seed.deserialize(ConfigValueDeserializer::new(
            value,
            self.key,
            self.options,
        ))
    }

    /// Deserializes a tuple variant payload.
    fn tuple_variant<V>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value.ok_or_else(|| {
            de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"tuple variant payload",
            )
        })?;
        de::Deserializer::deserialize_tuple(
            ConfigValueDeserializer::new(value, self.key, self.options),
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
            de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant payload",
            )
        })?;
        de::Deserializer::deserialize_struct(
            ConfigValueDeserializer::new(value, self.key, self.options),
            "",
            fields,
            visitor,
        )
    }
}
