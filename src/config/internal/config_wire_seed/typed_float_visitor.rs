// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Typed-float bridge that prevents backend parse errors from exposing input.

use serde::Deserializer;
use serde::de::Error as DeError;
use serde::de::Visitor;

use super::AccountingUniqueJsonValueSeed;

const INVALID_NATIVE_FLOAT: &str = "invalid native floating-point wire value";

/// Invokes a typed float deserializer while replacing untrusted early errors.
fn deserialize<'de, D>(
    deserializer: D,
    seed: AccountingUniqueJsonValueSeed<'_, '_>,
    float32: bool,
) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    let mut safe_failure = None;
    let visitor = TypedFloatVisitor {
        seed,
        safe_failure: &mut safe_failure,
    };
    let result = if float32 {
        deserializer.deserialize_f32(visitor)
    } else {
        deserializer.deserialize_f64(visitor)
    };

    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(D::Error::custom(
            safe_failure.as_deref().unwrap_or(INVALID_NATIVE_FLOAT),
        )),
    }
}

/// Calls the native Float32 deserializer with a bounded error bridge.
pub(super) fn deserialize_f32<'de, D>(
    deserializer: D,
    seed: AccountingUniqueJsonValueSeed<'_, '_>,
) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize(deserializer, seed, true)
}

/// Calls the native Float64 deserializer with a bounded error bridge.
pub(super) fn deserialize_f64<'de, D>(
    deserializer: D,
    seed: AccountingUniqueJsonValueSeed<'_, '_>,
) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize(deserializer, seed, false)
}

/// Forwards numeric events to the accounting seed without accepting other
/// types.
struct TypedFloatVisitor<'safe, 'transaction, 'budget> {
    /// Original seed responsible for type-role conversion and number charges.
    seed: AccountingUniqueJsonValueSeed<'transaction, 'budget>,
    /// Safe diagnostic produced only by the accounting seed's checked
    /// callbacks.
    safe_failure: &'safe mut Option<String>,
}

impl<'de, 'safe, 'transaction, 'budget> Visitor<'de> for TypedFloatVisitor<'safe, 'transaction, 'budget> {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a native JSON floating-point value")
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_f32::<E>(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_f64::<E>(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_i64::<E>(value))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_i128::<E>(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_u64::<E>(value))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.forward(|seed| seed.visit_u128::<E>(value))
    }
}

impl<'safe, 'transaction, 'budget> TypedFloatVisitor<'safe, 'transaction, 'budget> {
    /// Records only errors originating from the accounting seed's numeric
    /// callbacks.
    fn forward<E>(
        self,
        callback: impl FnOnce(AccountingUniqueJsonValueSeed<'transaction, 'budget>) -> Result<serde_json::Value, E>,
    ) -> Result<serde_json::Value, E>
    where
        E: DeError,
    {
        match callback(self.seed) {
            Ok(value) => Ok(value),
            Err(error) => {
                // These callbacks can fail only with safe conversion or budget summaries;
                // arbitrary deserializer errors never enter this method.
                *self.safe_failure = Some(error.to_string());
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use qubit_budget::json::JsonDecodeLimits;
    use serde::de::DeserializeSeed;

    use super::super::AccountingConfigWireSeed;
    use crate::Config;
    use crate::ConfigWireLimits;

    #[test]
    fn preserves_safe_number_budget_error_from_numeric_callback() {
        let mut config = Config::new();
        config.set("ratio", 1.25_f64).unwrap();
        let bytes = serde_json::to_vec(&config).unwrap();
        let limits = ConfigWireLimits::builder_from(&ConfigWireLimits::default())
            .json_decode(JsonDecodeLimits::builder().max_number_bytes(2).build())
            .build();
        let mut deserializer = serde_json::Deserializer::from_slice(&bytes);

        let error = match AccountingConfigWireSeed::new(limits).deserialize(&mut deserializer) {
            Ok(_) => panic!("the float should exceed the configured number-byte budget"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("NumberBytes"), "unexpected error: {error}");
        assert!(error.to_string().contains("exactly 4"), "unexpected charge: {error}");
        assert!(!error.to_string().contains("invalid native floating-point wire value"));
    }
}
