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

/// Provides a borrowed external enum tag and its payload deserializer.
///
/// The payload keeps the same conversion session as the containing structured
/// read, so nested variant fields share its accounting and limits.
#[must_use = "the enum access must be consumed by a Serde visitor"]
pub(super) struct SourceEnumAccess<'tree, 'policy, 'session> {
    /// Borrowed variant name from the prepared source tree.
    pub(super) variant: &'tree str,
    /// Deserializer for the variant payload, carrying its root-relative path
    /// and the shared read session.
    pub(super) payload: SourceDeserializer<'tree, 'policy, 'session>,
}

impl<'de, 'tree, 'policy: 'tree, 'session> EnumAccess<'de> for SourceEnumAccess<'tree, 'policy, 'session> {
    /// Conversion error produced while reading the source enum.
    type Error = ConfigDeserializeError;
    /// Deserializer that reads the selected variant's payload.
    type Variant = SourceDeserializer<'tree, 'policy, 'session>;

    /// Deserializes the borrowed variant tag and returns its payload source.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Seed that constructs the target enum discriminant.
    ///
    /// # Parameters
    ///
    /// * `seed` - Visitor that converts the source variant name.
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed
            .deserialize(StrDeserializer::<ConfigDeserializeError>::new(self.variant))
            .map_err(|error| error.with_path(self.payload.path.clone()))?;
        Ok((variant, self.payload))
    }
}

impl<'de, 'tree, 'policy: 'tree> VariantAccess<'de> for SourceDeserializer<'tree, 'policy, '_> {
    /// Conversion error returned by the source-backed payload deserializer.
    type Error = ConfigDeserializeError;

    /// Completes deserialization of a unit variant payload.
    fn unit_variant(self) -> Result<(), Self::Error> {
        Deserialize::deserialize(self)
    }
    /// Deserializes the payload of a newtype variant.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Seed that converts the newtype payload.
    ///
    /// # Parameters
    ///
    /// * `seed` - Visitor that constructs the newtype value.
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Self::Error> {
        let path = self.path.clone();
        seed.deserialize(self).map_err(|error| error.with_path(path))
    }
    /// Deserializes a tuple variant from the borrowed payload sequence.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the tuple variant value.
    ///
    /// # Parameters
    ///
    /// * `len` - Required tuple element count.
    /// * `visitor` - Visitor receiving the tuple elements.
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        let path = self.path.clone();
        de::Deserializer::deserialize_tuple(self, len, visitor).map_err(|error| error.with_path(path))
    }
    /// Deserializes a struct variant from the borrowed payload map.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the struct variant value.
    ///
    /// # Parameters
    ///
    /// * `fields` - Target struct field names accepted by the visitor.
    /// * `visitor` - Visitor receiving the variant's map payload.
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let path = self.path.clone();
        de::Deserializer::deserialize_struct(self, "", fields, visitor).map_err(|error| error.with_path(path))
    }
}
