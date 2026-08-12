// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Budget-aware seed for persisted configuration wire values.

use qubit_budget::JsonValueBudget;
use qubit_json::BudgetedJsonValueSeed;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeSeed;
use serde::de::Error as _;
use serde_json::Value;
use serde_json::from_value;

use super::ConfigWire;
use super::ConfigWireFields;
use crate::ConfigWireDecodeError;
use crate::ConfigWireLimitKind;
use crate::ConfigWireLimits;

/// Decodes one Config wire value under explicit domain limits.
pub(in crate::config) struct ConfigWireSeed {
    limits: ConfigWireLimits,
    account_decoded_value: bool,
}

impl ConfigWireSeed {
    /// Creates a seed that budgets decoded Serde events.
    pub(in crate::config) const fn new(limits: ConfigWireLimits) -> Self {
        Self {
            limits,
            account_decoded_value: true,
        }
    }

    /// Creates a seed for input already admitted by a JSON decode session.
    pub(in crate::config) const fn preaccounted(
        limits: ConfigWireLimits,
    ) -> Self {
        Self {
            limits,
            account_decoded_value: false,
        }
    }

    /// Checks the configuration-specific property dimensions.
    fn check_properties<'a>(
        &self,
        count: usize,
        keys: impl Iterator<Item = &'a str>,
    ) -> Result<(), ConfigWireDecodeError> {
        let count =
            u64::try_from(count).expect("property count must fit in u64");
        self.limits
            .properties_limit()
            .check(count)
            .map_err(|error| ConfigWireDecodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                value: error
                    .exact_observed()
                    .expect("point failure carries an exact value"),
                maximum: error
                    .maximum()
                    .expect("point failure carries a maximum"),
            })?;
        for key in keys {
            let bytes = u64::try_from(key.len())
                .expect("property key length must fit in u64");
            self.limits
                .property_key_bytes_limit()
                .check(bytes)
                .map_err(|error| ConfigWireDecodeError::LimitExceeded {
                    kind: ConfigWireLimitKind::PropertyKeyBytes,
                    value: error
                        .exact_observed()
                        .expect("point failure carries an exact value"),
                    maximum: error
                        .maximum()
                        .expect("point failure carries a maximum"),
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
        self.check_properties(
            properties.len(),
            properties.keys().map(String::as_str),
        )
    }

    /// Checks property dimensions on directly decoded wire fields.
    fn check_fields(
        &self,
        fields: &ConfigWireFields,
    ) -> Result<(), ConfigWireDecodeError> {
        self.check_properties(
            fields.properties.len(),
            fields.properties.keys().map(String::as_str),
        )
    }
}

impl<'de> DeserializeSeed<'de> for ConfigWireSeed {
    type Value = Result<ConfigWire, ConfigWireDecodeError>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = if self.account_decoded_value {
            let mut budget =
                JsonValueBudget::new(self.limits.json_decode().value_limits());
            let value = BudgetedJsonValueSeed::new(&mut budget)
                .deserialize(deserializer)?;
            if let Err(error) = self.check_value(&value) {
                return Ok(Err(error));
            }
            from_value(value).map_err(D::Error::custom)?
        } else {
            let fields = ConfigWireFields::deserialize(deserializer)?;
            if let Err(error) = self.check_fields(&fields) {
                return Ok(Err(error));
            }
            fields
        };
        Ok(ConfigWire::from_fields(fields)
            .map_err(ConfigWireDecodeError::InvalidConfig))
    }
}
