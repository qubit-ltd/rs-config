// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde enum access over a configuration value.

use serde::de::{DeserializeSeed, EnumAccess, IntoDeserializer, value::StringDeserializer};
use serde_json::Value;

use super::config_variant_access::ConfigVariantAccess;
use crate::config_deserialize_error::ConfigDeserializeError;
use crate::options::ReadPolicy;

/// Enum access over a configuration value.
pub(in crate::config_value_deserializer) struct ConfigEnumAccess<'a> {
    variant: String,
    value: Option<Value>,
    key: String,
    options: &'a ReadPolicy,
}

impl<'a> ConfigEnumAccess<'a> {
    /// Creates enum access for a variant and optional payload.
    pub(in crate::config_value_deserializer) fn new(
        variant: String,
        value: Option<Value>,
        key: String,
        options: &'a ReadPolicy,
    ) -> Self {
        Self {
            variant,
            value,
            key,
            options,
        }
    }
}

impl<'de, 'a> EnumAccess<'de> for ConfigEnumAccess<'a> {
    type Error = ConfigDeserializeError;
    type Variant = ConfigVariantAccess<'a>;

    /// Deserializes the enum variant identifier.
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let child_key = if self.key.is_empty() {
            self.variant.clone()
        } else {
            format!("{}.{}", self.key, self.variant)
        };
        let variant_deserializer: StringDeserializer<Self::Error> =
            self.variant.into_deserializer();
        let variant = seed.deserialize(variant_deserializer)?;
        Ok((
            variant,
            ConfigVariantAccess::new(self.value, child_key, self.options),
        ))
    }
}
