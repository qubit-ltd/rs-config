// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decodes configuration wire values after JSON-session admission, without
//! charging the admitted JSON budget a second time.

use qubit_json::value::DuplicateKeyRejectingJsonValueSeed;
use serde::Deserializer;
use serde::de::DeserializeSeed;

use super::ConfigWire;
use super::accounting_config_wire_seed::AccountingConfigWireSeed;
use crate::ConfigWireDecodeError;
use crate::ConfigWireLimits;

/// Decodes a wire value whose JSON tree was admitted by a decode session.
pub(in crate::config) struct JsonAdmittedConfigWireSeed {
    /// Configuration-specific limits applied after JSON admission.
    limits: ConfigWireLimits,
}

impl JsonAdmittedConfigWireSeed {
    /// Creates a seed for input already admitted by a JSON decode session.
    ///
    /// # Parameters
    /// - `limits`: Configuration-specific limits to apply after JSON admission.
    ///
    /// # Returns
    /// A seed that converts the admitted value without charging its JSON budget
    /// again.
    pub(in crate::config) const fn new(limits: ConfigWireLimits) -> Self {
        Self { limits }
    }
}

impl<'de> DeserializeSeed<'de> for JsonAdmittedConfigWireSeed {
    /// The configuration wire result, including configuration-specific decode
    /// failures.
    type Value = Result<ConfigWire, ConfigWireDecodeError>;

    /// Converts an admitted JSON value into the configuration wire
    /// representation.
    ///
    /// # Parameters
    /// - `deserializer`: Deserializer for the admitted JSON value.
    ///
    /// # Returns
    /// The nested wire conversion result; the outer result reports deserializer
    /// failures.
    ///
    /// # Errors
    /// Returns the deserializer error if duplicate-key checking or JSON value
    /// decoding fails.
    ///
    /// # Type Parameters
    ///
    /// * `D` - Deserializer implementation reading the admitted JSON value.
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = DuplicateKeyRejectingJsonValueSeed::new()
            .deserialize(deserializer)?
            .into_inner();
        AccountingConfigWireSeed::new(self.limits).decode_value::<D::Error>(value)
    }
}
