// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow multiple-public-types
//! Budget-aware seed for persisted configuration wire values.

use std::collections::HashSet;

use qubit_budget::json::JsonContainerKind;
use qubit_budget::json::JsonMeasurement;
use qubit_budget::json::JsonResource;
use qubit_budget::json::JsonValueBudget;
use qubit_budget::json::JsonValueTransaction;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeSeed;
use serde::de::Error as DeError;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use serde_json::from_value;

use super::ConfigWire;
use super::ConfigWireFields;
use crate::ConfigWireDecodeError;
use crate::ConfigWireLimitKind;
use crate::ConfigWireLimits;

/// Decodes one Config wire value under explicit domain limits.
pub(in crate::config) struct AccountingConfigWireSeed {
    /// Configuration-specific limits applied during decoding.
    limits: ConfigWireLimits,
}

impl AccountingConfigWireSeed {
    /// Creates a seed that budgets decoded Serde events.
    pub(in crate::config) const fn new(limits: ConfigWireLimits) -> Self {
        Self { limits }
    }
}

/// Decodes a wire value whose JSON tree was admitted by a decode session.
pub(in crate::config) struct JsonAdmittedConfigWireSeed {
    /// Configuration-specific limits applied after JSON admission.
    limits: ConfigWireLimits,
}

impl JsonAdmittedConfigWireSeed {
    /// Creates a seed for input already admitted by a JSON decode session.
    pub(in crate::config) const fn new(limits: ConfigWireLimits) -> Self {
        Self { limits }
    }
}

impl AccountingConfigWireSeed {
    /// Checks the configuration-specific property dimensions.
    fn check_properties<'a>(
        &self,
        count: usize,
        keys: impl Iterator<Item = &'a str>,
    ) -> Result<(), ConfigWireDecodeError> {
        let count = u64::try_from(count).expect("property count must fit in u64");
        if let Err(error) = self.limits.properties_limit().check(count) {
            return Err(ConfigWireDecodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                value: error.exact_observed().expect("point failure carries an exact value"),
                maximum: error.maximum(),
            });
        }
        for key in keys {
            let bytes = u64::try_from(key.len()).expect("property key length must fit in u64");
            self.limits.property_key_bytes_limit().check(bytes).map_err(|error| {
                ConfigWireDecodeError::LimitExceeded {
                    kind: ConfigWireLimitKind::PropertyKeyBytes,
                    value: error.exact_observed().expect("point failure carries an exact value"),
                    maximum: error.maximum(),
                }
            })?;
        }
        Ok(())
    }

    /// Checks property dimensions before typed Property materialization.
    fn check_value(&self, value: &Value) -> Result<(), ConfigWireDecodeError> {
        let Some(properties) = value
            .as_object()
            .and_then(|object| object.get("properties"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        self.check_properties(properties.len(), properties.keys().map(String::as_str))
    }
}

/// Seed that builds a JSON value while accounting events incrementally and
/// rejecting duplicate object keys.
struct AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    /// Transaction receiving incremental JSON resource charges.
    transaction: &'transaction mut JsonValueTransaction<'budget, JsonResource, u64>,
    /// Root-inclusive nesting depth of the value being decoded.
    depth: usize,
    /// Container count to check before decoding the next child.
    prospective_container: Option<(JsonContainerKind, usize)>,
}

impl<'transaction, 'budget> AccountingUniqueJsonValueSeed<'transaction, 'budget> {
    /// Creates a root seed using the supplied accounting transaction.
    fn new(transaction: &'transaction mut JsonValueTransaction<'budget, JsonResource, u64>) -> Self {
        Self {
            transaction,
            depth: 1,
            prospective_container: None,
        }
    }

    /// Creates a child seed at the next root-inclusive depth.
    fn child<'child>(&'child mut self) -> AccountingUniqueJsonValueSeed<'child, 'budget> {
        AccountingUniqueJsonValueSeed {
            transaction: self.transaction,
            depth: self.depth.saturating_add(1),
            prospective_container: None,
        }
    }

    /// Creates a child seed that checks a prospective container member before
    /// deserializing its value.
    fn prospective_child<'child>(
        &'child mut self,
        kind: JsonContainerKind,
        prospective: usize,
    ) -> AccountingUniqueJsonValueSeed<'child, 'budget> {
        AccountingUniqueJsonValueSeed {
            transaction: self.transaction,
            depth: self.depth.saturating_add(1),
            prospective_container: Some((kind, prospective)),
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
        deserializer.deserialize_any(self)
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
        let number =
            Number::from_i128(value).ok_or_else(|| E::custom("JSON integer is outside the supported 64-bit range"))?;
        self.enter_number(number)
    }

    fn visit_u128<E>(mut self, value: u128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let number =
            Number::from_u128(value).ok_or_else(|| E::custom("JSON integer is outside the supported 64-bit range"))?;
        self.enter_number(number)
    }

    fn visit_f64<E>(mut self, value: f64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        let number =
            Number::from_f64(value).ok_or_else(|| E::custom("non-finite float is not representable as JSON"))?;
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
        deserializer.deserialize_any(self)
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
            let Some(value) = sequence.next_element_seed(self.prospective_child(JsonContainerKind::Sequence, next))?
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
        let mut seen = HashSet::new();
        let mut entries = 0_usize;
        while let Some(key) = map.next_key::<String>()? {
            let Some(next) = entries.checked_add(1) else {
                return Err(A::Error::custom("JSON map entry count overflowed usize"));
            };
            self.transaction
                .check_container_count(JsonContainerKind::Map, next)
                .map_err(A::Error::custom)?;
            if !seen.insert(key.clone()) {
                return Err(A::Error::custom("duplicate JSON object key"));
            }
            self.transaction
                .try_admit(JsonMeasurement::Key { bytes: key.len() })
                .map_err(A::Error::custom)?;
            let value = map.next_value_seed(self.child())?;
            values.insert(key, value);
            entries = next;
        }
        Ok(Value::Object(values))
    }
}

impl JsonAdmittedConfigWireSeed {
    /// Checks property dimensions on directly decoded wire fields.
    fn check_fields(&self, fields: &ConfigWireFields) -> Result<(), ConfigWireDecodeError> {
        let count = u64::try_from(fields.properties.len()).expect("property count must fit in u64");
        self.limits
            .properties_limit()
            .check(count)
            .map_err(|error| ConfigWireDecodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                value: error.exact_observed().expect("point failure carries an exact value"),
                maximum: error.maximum(),
            })?;
        for key in fields.properties.keys() {
            let bytes = u64::try_from(key.len()).expect("property key length must fit in u64");
            self.limits.property_key_bytes_limit().check(bytes).map_err(|error| {
                ConfigWireDecodeError::LimitExceeded {
                    kind: ConfigWireLimitKind::PropertyKeyBytes,
                    value: error.exact_observed().expect("point failure carries an exact value"),
                    maximum: error.maximum(),
                }
            })?;
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for AccountingConfigWireSeed {
    type Value = Result<ConfigWire, ConfigWireDecodeError>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut budget = JsonValueBudget::new(*self.limits.json_decode().value_limits());
        let mut transaction = budget.transaction();
        let value = AccountingUniqueJsonValueSeed::new(&mut transaction).deserialize(deserializer)?;
        transaction.commit().map_err(D::Error::custom)?;
        if let Err(error) = self.check_value(&value) {
            return Ok(Err(error));
        }
        let fields = from_value(value).map_err(D::Error::custom)?;
        Ok(ConfigWire::from_fields(fields).map_err(ConfigWireDecodeError::InvalidConfig))
    }
}

impl<'de> DeserializeSeed<'de> for JsonAdmittedConfigWireSeed {
    type Value = Result<ConfigWire, ConfigWireDecodeError>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ConfigWireFields::deserialize(deserializer)?;
        if let Err(error) = self.check_fields(&fields) {
            return Ok(Err(error));
        }
        Ok(ConfigWire::from_fields(fields).map_err(ConfigWireDecodeError::InvalidConfig))
    }
}
