// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! External enum tags borrow their payload and preserve variant paths.

use serde::Deserialize;
use serde::de;
use serde::de::DeserializeSeed;
use serde::de::EnumAccess;
use serde::de::VariantAccess;
use serde::de::Visitor;
use serde::de::value::StrDeserializer;

use super::deserializer::SourceDeserializer;
use crate::config_deserialize_error::ConfigDeserializeError;

pub(super) struct SourceEnumAccess<'tree, 'policy, 'session> {
    pub(super) variant: &'tree str,
    pub(super) payload: SourceDeserializer<'tree, 'policy, 'session>,
}

impl<'de, 'tree, 'policy: 'tree, 'session> EnumAccess<'de> for SourceEnumAccess<'tree, 'policy, 'session> {
    type Error = ConfigDeserializeError;
    type Variant = SourceDeserializer<'tree, 'policy, 'session>;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed
            .deserialize(StrDeserializer::<ConfigDeserializeError>::new(self.variant))
            .map_err(|error| error.with_path(self.payload.path.clone()))?;
        Ok((variant, self.payload))
    }
}

impl<'de, 'tree, 'policy: 'tree> VariantAccess<'de> for SourceDeserializer<'tree, 'policy, '_> {
    type Error = ConfigDeserializeError;
    fn unit_variant(self) -> Result<(), Self::Error> {
        Deserialize::deserialize(self)
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Self::Error> {
        let path = self.path.clone();
        seed.deserialize(self).map_err(|error| error.with_path(path))
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        let path = self.path.clone();
        de::Deserializer::deserialize_tuple(self, len, visitor).map_err(|error| error.with_path(path))
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let path = self.path.clone();
        de::Deserializer::deserialize_struct(self, "", fields, visitor).map_err(|error| error.with_path(path))
    }
}
