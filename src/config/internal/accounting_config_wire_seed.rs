// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Budget-aware seed for persisted configuration wire values.

use qubit_budget::json::JsonValueBudget;
use serde::Deserializer;
use serde::de::DeserializeSeed;
use serde::de::Error as DeError;
use serde_json::Value;

use super::ConfigWire;
use super::decode_fields;
use crate::ConfigWireDecodeError;
use crate::ConfigWireLimitKind;
use crate::ConfigWireLimits;

// Implements incremental JSON event admission and duplicate-key rejection.
mod accounting_unique_json_value_seed;
// Bridges typed native floats to safe accounting diagnostics.
mod typed_float_visitor;

use self::accounting_unique_json_value_seed::AccountingUniqueJsonValueSeed;

/// Decodes one Config wire value under explicit domain limits.
pub(in crate::config) struct AccountingConfigWireSeed {
    /// Configuration-specific limits applied during decoding.
    limits: ConfigWireLimits,
}

impl AccountingConfigWireSeed {
    /// Creates a seed that budgets decoded Serde events.
    ///
    /// # Parameters
    ///
    /// * `limits` - Shared JSON and configuration-specific decode limits.
    ///
    /// # Returns
    ///
    /// A seed that applies these limits while reading one persisted config.
    pub(in crate::config) const fn new(limits: ConfigWireLimits) -> Self {
        Self { limits }
    }
}

impl AccountingConfigWireSeed {
    /// Checks the configuration-specific property dimensions.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of decoded properties.
    /// * `keys` - Property names whose encoded byte lengths must be admitted.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the count and every key fit their configured limits.
    ///
    /// # Errors
    ///
    /// Returns the first exceeded configuration-specific property limit.
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
    ///
    /// # Parameters
    ///
    /// * `value` - Parsed JSON value containing an optional properties object.
    ///
    /// # Returns
    ///
    /// `Ok(())` when property limits pass or the value has no properties
    /// object.
    ///
    /// # Errors
    ///
    /// Returns the first exceeded property count or key-byte limit.
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

    /// Checks domain limits before constructing typed properties from a wire
    /// value.
    ///
    /// # Parameters
    ///
    /// * `value` - Parsed JSON value to admit and convert to a wire contract.
    ///
    /// # Returns
    ///
    /// The outer result is always `Ok`; its inner result is the decoded wire
    /// contract or a configuration decode error.
    ///
    /// # Errors
    ///
    /// The inner result contains property-limit, field-decoding, or invalid
    /// wire-version errors. The generic outer error type is retained for the
    /// `DeserializeSeed` call site's type inference.
    ///
    /// # Type Parameters
    ///
    /// * `E` - Serde error type used for the outer deserialization result.
    pub(super) fn decode_value<E>(&self, value: Value) -> Result<Result<ConfigWire, ConfigWireDecodeError>, E>
    where
        E: DeError,
    {
        if let Err(error) = self.check_value(&value) {
            return Ok(Err(error));
        }
        let fields = match decode_fields(value) {
            Ok(fields) => fields,
            Err(error) => return Ok(Err(error)),
        };
        Ok(ConfigWire::from_fields(fields).map_err(ConfigWireDecodeError::InvalidConfig))
    }
}

impl<'de> DeserializeSeed<'de> for AccountingConfigWireSeed {
    /// The decoded wire contract or its configuration-specific validation
    /// error.
    type Value = Result<ConfigWire, ConfigWireDecodeError>;

    /// Deserializes one wire value while charging JSON and domain limits.
    ///
    /// # Parameters
    ///
    /// * `deserializer` - Serde source for one persisted configuration.
    ///
    /// # Returns
    ///
    /// The decoded wire result after the budget transaction commits.
    ///
    /// # Errors
    ///
    /// Returns the deserializer or budget-transaction error outside the inner
    /// result; domain and wire validation errors are carried inside it.
    ///
    /// # Type Parameters
    ///
    /// * `D` - Deserializer implementation reading the persisted wire value.
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut budget = JsonValueBudget::new(*self.limits.json_decode().value_limits());
        let mut transaction = budget.transaction();
        let value = AccountingUniqueJsonValueSeed::new(&mut transaction).deserialize(deserializer)?;
        transaction.commit().map_err(D::Error::custom)?;
        self.decode_value::<D::Error>(value)
    }
}
