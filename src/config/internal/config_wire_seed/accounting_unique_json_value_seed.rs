// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builds JSON values while charging each decoded event and rejecting duplicate
//! keys.
//!
//! The caller owns the accounting transaction and commits it after the root
//! value succeeds. Typed-float errors are replaced with safe summaries; only
//! errors recorded by this seed's checked callbacks are retained.

use qubit_budget::json::JsonContainerKind;
use qubit_budget::json::JsonMeasurement;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueTransaction;
use serde::Deserializer;
use serde::de::DeserializeSeed;
use serde::de::Error as DeError;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;

use super::super::wire_read_role::WireReadRole;
use super::typed_float_visitor;

/// Seed that builds a JSON value while accounting events incrementally and
/// rejecting duplicate object keys.
pub(super) struct AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    /// Transaction receiving incremental JSON resource charges.
    pub(super) transaction: &'transaction mut JsonValueTransaction<'budget, JsonResource, u64>,
    /// Root-inclusive nesting depth of the value being decoded.
    pub(super) depth: usize,
    /// Container count to check before decoding the next child.
    pub(super) prospective_container: Option<(JsonContainerKind, usize)>,
    /// Schema location used to request typed float values only in the wire.
    pub(super) role: WireReadRole,
}

impl<'transaction, 'budget> AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    /// Creates a root seed using the supplied accounting transaction.
    pub(super) fn new(transaction: &'transaction mut JsonValueTransaction<'budget, JsonResource, u64>) -> Self {
        Self {
            transaction,
            depth: 1,
            prospective_container: None,
            role: WireReadRole::ConfigRoot,
        }
    }

    /// Creates a child seed at the next root-inclusive depth.
    fn child<'child>(&'child mut self, role: WireReadRole) -> AccountingUniqueJsonValueSeed<'child, 'budget> {
        AccountingUniqueJsonValueSeed {
            transaction: self.transaction,
            depth: self.depth.saturating_add(1),
            prospective_container: None,
            role,
        }
    }

    /// Creates a child seed that checks a prospective container member before
    /// deserializing its value.
    fn prospective_child<'child>(
        &'child mut self,
        kind: JsonContainerKind,
        prospective: usize,
        role: WireReadRole,
    ) -> AccountingUniqueJsonValueSeed<'child, 'budget> {
        AccountingUniqueJsonValueSeed {
            transaction: self.transaction,
            depth: self.depth.saturating_add(1),
            prospective_container: Some((kind, prospective)),
            role,
        }
    }

    /// Accounts a JSON number while retaining serde_json's representable range.
    fn enter_number<E>(&mut self, number: Number) -> Result<Value, E>
    where
        E: DeError,
    {
        self.transaction
            .try_admit(JsonMeasurement::Number {
                depth: self.depth,
                bytes: number.to_string().len(),
            })
            .map_err(E::custom)?;
        Ok(Value::Number(number))
    }
}

impl<'de, 'transaction, 'budget> DeserializeSeed<'de> for AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Some((kind, prospective)) = self.prospective_container {
            self.transaction
                .check_container_count(kind, prospective)
                .map_err(D::Error::custom)?;
        }
        match self.role {
            WireReadRole::Float32 | WireReadRole::Float64 => deserializer.deserialize_option(self),
            _ => deserializer.deserialize_any(self),
        }
    }
}

impl<'de, 'transaction, 'budget> Visitor<'de> for AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value with unique object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.transaction
            .try_admit(JsonMeasurement::Boolean { depth: self.depth })
            .map_err(E::custom)?;
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(mut self, value: i64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.enter_number(Number::from(value))
    }

    fn visit_u64<E>(mut self, value: u64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.enter_number(Number::from(value))
    }

    fn visit_i128<E>(mut self, value: i128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let number = if let Ok(value) = i64::try_from(value) {
            Number::from(value)
        } else {
            Number::from(
                u64::try_from(value).map_err(|_| E::custom("JSON integer is outside the supported 64-bit range"))?,
            )
        };
        self.enter_number(number)
    }

    fn visit_u128<E>(mut self, value: u128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let value =
            u64::try_from(value).map_err(|_| E::custom("JSON integer is outside the supported 64-bit range"))?;
        self.enter_number(Number::from(value))
    }

    fn visit_f64<E>(mut self, value: f64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let value = match self.role {
            WireReadRole::Float32 => f64::from(value as f32),
            _ => value,
        };
        let number =
            Number::from_f64(value).ok_or_else(|| E::custom("non-finite float is not representable as JSON"))?;
        self.enter_number(number)
    }

    fn visit_f32<E>(mut self, value: f32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let number = Number::from_f64(f64::from(value))
            .ok_or_else(|| E::custom("non-finite float is not representable as JSON"))?;
        self.enter_number(number)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.transaction
            .try_admit(JsonMeasurement::String {
                depth: self.depth,
                bytes: value.len(),
            })
            .map_err(E::custom)?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.transaction
            .try_admit(JsonMeasurement::String {
                depth: self.depth,
                bytes: value.len(),
            })
            .map_err(E::custom)?;
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.transaction
            .try_admit(JsonMeasurement::Null { depth: self.depth })
            .map_err(E::custom)?;
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        match self.role {
            WireReadRole::Float32 => typed_float_visitor::deserialize_f32(deserializer, self),
            WireReadRole::Float64 => typed_float_visitor::deserialize_f64(deserializer, self),
            _ => deserializer.deserialize_any(self),
        }
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(mut self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.transaction
            .try_enter_container(JsonContainerKind::Sequence, self.depth)
            .map_err(A::Error::custom)?;
        let mut values = Vec::new();
        loop {
            let Some(next) = values.len().checked_add(1) else {
                return Err(A::Error::custom("JSON sequence item count overflowed usize"));
            };
            let element_role = self.role.sequence_element();
            let Some(value) =
                sequence.next_element_seed(self.prospective_child(JsonContainerKind::Sequence, next, element_role))?
            else {
                break;
            };
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.transaction
            .try_enter_container(JsonContainerKind::Map, self.depth)
            .map_err(A::Error::custom)?;
        let mut values = Map::new();
        let mut entries = 0_usize;
        while let Some(key) = map.next_key::<String>()? {
            let Some(next) = entries.checked_add(1) else {
                return Err(A::Error::custom("JSON map entry count overflowed usize"));
            };
            self.transaction
                .check_container_count(JsonContainerKind::Map, next)
                .map_err(A::Error::custom)?;
            if values.contains_key(&key) {
                return Err(A::Error::custom("duplicate JSON object key"));
            }
            self.transaction
                .try_admit(JsonMeasurement::Key { bytes: key.len() })
                .map_err(A::Error::custom)?;
            let role = self.role.child_for_key(&key);
            let value = map.next_value_seed(self.child(role))?;
            values.insert(key, value);
            entries = next;
        }
        Ok(Value::Object(values))
    }
}
