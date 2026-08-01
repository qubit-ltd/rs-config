// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for typed `FromConfig` parsing behavior.

use std::time::Duration;

#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
#[cfg(feature = "chrono")]
use chrono::{
    NaiveDate,
    NaiveDateTime,
};
#[cfg(feature = "num-bigint")]
use num_bigint::BigInt;
use qubit_config::{
    Config,
    ConfigError,
    options::ReadPolicy,
};
use qubit_datatype::{
    InvalidValueReason,
    NumericConversionOptions,
};
#[cfg(any(feature = "bigdecimal", feature = "num-bigint"))]
use std::str::FromStr;

#[test]
fn test_from_config_converts_scalar_string_to_duration() {
    let mut config = Config::new();
    config
        .set("server.timeout_secs", "30ms")
        .expect("setting config value should succeed");

    let timeout = config
        .get::<Duration>("server.timeout_secs")
        .expect("duration should parse from scalar string");

    assert_eq!(timeout, Duration::from_millis(30));
}

#[test]
fn test_from_config_reports_keyed_conversion_error() {
    let mut config = Config::new();
    config
        .set("server.port", "not-a-port")
        .expect("setting config value should succeed");

    let error = config
        .get::<u16>("server.port")
        .expect_err("invalid integer should fail conversion");

    assert!(matches!(
        error,
        ConfigError::ConversionError { key, .. } if key == "server.port"
    ));
}

/// Test typed vectors are converted without a lossy string intermediary.
#[test]
fn test_from_config_preserves_typed_vector_values() {
    let durations = vec![Duration::new(1, 1), Duration::new(2, 999_999_999)];
    let chars = vec!['A', '中'];
    let mut config = Config::new();
    config.set("chars", chars.clone()).expect("set chars");
    config
        .set("durations", durations.clone())
        .expect("set durations");

    assert_eq!(config.get::<Vec<char>>("chars").unwrap(), chars);
    assert_eq!(config.get::<Vec<Duration>>("durations").unwrap(), durations,);
}

/// Test chrono vectors preserve their original representations.
#[cfg(feature = "chrono")]
#[test]
fn test_from_config_preserves_chrono_vector_values() {
    let dates = vec![NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()];
    let datetimes = vec![
        NaiveDateTime::parse_from_str(
            "2026-07-13T01:02:03.123456789",
            "%Y-%m-%dT%H:%M:%S%.f",
        )
        .unwrap(),
    ];
    let mut config = Config::new();
    config.set("dates", dates.clone()).unwrap();
    config.set("datetimes", datetimes.clone()).unwrap();

    assert_eq!(config.get::<Vec<NaiveDate>>("dates").unwrap(), dates);
    assert_eq!(
        config.get::<Vec<NaiveDateTime>>("datetimes").unwrap(),
        datetimes,
    );
}

/// Test big integer vectors preserve their original representations.
#[cfg(feature = "num-bigint")]
#[test]
fn test_from_config_preserves_big_integer_vector_values() {
    let integers = vec![BigInt::from_str("123456789012345678901").unwrap()];
    let mut config = Config::new();
    config.set("integers", integers.clone()).unwrap();

    assert_eq!(config.get::<Vec<BigInt>>("integers").unwrap(), integers);
}

/// Test big decimal vectors preserve their original representations.
#[cfg(feature = "bigdecimal")]
#[test]
fn test_from_config_preserves_big_decimal_vector_values() {
    let decimals = vec![BigDecimal::from_str("1.234567890123456789").unwrap()];
    let mut config = Config::new();
    config.set("decimals", decimals.clone()).unwrap();

    assert_eq!(config.get::<Vec<BigDecimal>>("decimals").unwrap(), decimals);
}

/// Test list failures retain the original source index and structured reason.
#[test]
fn test_from_config_list_error_preserves_source_index() {
    let mut config = Config::new();
    config
        .set("ports", vec!["1", "bad", "3"])
        .expect("set ports");

    assert!(matches!(
        config.get::<Vec<u16>>("ports"),
        Err(ConfigError::ConversionError {
            key,
            source_index: Some(1),
            source,
        }) if key == "ports"
            && matches!(
                source.reason(),
                Some(InvalidValueReason::InvalidSyntax { .. }),
            )
    ));
}

/// Test exact conversion is default and lossy conversion is explicit.
#[test]
fn test_from_config_numeric_options_are_explicit() {
    let mut config = Config::new();
    config.set("values", vec![1.5f64, 2.9]).expect("set values");
    assert!(matches!(
        config.get::<Vec<i32>>("values"),
        Err(ConfigError::ConversionError {
            source,
            ..
        }) if matches!(
            source.reason(),
            Some(InvalidValueReason::PrecisionLoss),
        )
    ));

    config.set_default_read_policy(
        ReadPolicy::default()
            .with_numeric_options(NumericConversionOptions::lossy()),
    );
    assert_eq!(config.get::<Vec<i32>>("values").unwrap(), vec![1, 2]);
}
