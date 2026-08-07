// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[allow(unused_imports)]
use crate::{
    Config, ConfigError, DataConversionError, DataType, Deserialize, DurationConversionOptions,
    DurationRoundingPolicy, DurationUnit, HashMap, InvalidValueReason, MultiValues, Property,
    ReadPolicy, Value,
};
#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
#[cfg(feature = "num-bigint")]
use num_bigint::BigInt;
#[cfg(feature = "bigdecimal")]
use std::str::FromStr;
use std::time::Duration;
#[cfg(feature = "url")]
use url::Url;

#[derive(Deserialize, Debug, PartialEq)]
struct AnyStruct {
    val: serde_json::Value,
}

/// Builds a config containing one collection-shaped property.
fn config_with_mv(key: &str, mv: MultiValues) -> Config {
    let mut config = Config::new();
    let property = Property::new(key, mv).expect("the test property key should be canonical");
    config.insert_property(key, property).unwrap();
    config
}

/// Builds a config containing one scalar-shaped property.
fn config_with_value(key: &str, value: Value) -> Config {
    let mut config = Config::new();
    let property = Property::new(key, value).expect("the test property key should be canonical");
    config.insert_property(key, property).unwrap();
    config
}

#[test]
fn test_deserialize_bool_single() {
    let mut config = Config::new();
    config.set("x.val", true).unwrap();
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::Value::Bool(true));
}

#[test]
fn test_deserialize_bool_multi() {
    let mut config = Config::new();
    config.set("x.val", vec![true, false]).unwrap();
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_array());
}

#[test]
fn test_deserialize_int8() {
    let config = config_with_value("x.val", Value::Int8(42i8));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(42));
}

#[test]
fn test_deserialize_int16() {
    let config = config_with_value("x.val", Value::Int16(1000i16));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(1000));
}

#[test]
fn test_deserialize_int32() {
    let config = config_with_value("x.val", Value::Int32(8080i32));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(8080));
}

#[test]
fn test_deserialize_int64() {
    let config = config_with_value("x.val", Value::Int64(9999i64));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(9999));
}

#[test]
fn test_deserialize_uint8() {
    let config = config_with_value("x.val", Value::UInt8(255u8));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(255));
}

#[test]
fn test_deserialize_uint16() {
    let config = config_with_value("x.val", Value::UInt16(1000u16));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(1000));
}

#[test]
fn test_deserialize_uint32() {
    let config = config_with_value("x.val", Value::UInt32(42u32));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(42));
}

#[test]
fn test_deserialize_uint64() {
    let config = config_with_value("x.val", Value::UInt64(42u64));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!(42));
}

#[test]
fn test_deserialize_float32() {
    let config = config_with_value("x.val", Value::Float32(1.5f32));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_number());
}

#[test]
fn test_deserialize_float64() {
    let config = config_with_value("x.val", Value::Float64(1.5f64));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_number());
}

#[test]
fn test_deserialize_float32_nan_is_rejected() {
    let config = config_with_mv("x.val", MultiValues::Float32(vec![f32::NAN]));
    let error = config.deserialize::<AnyStruct>("x").unwrap_err();

    let ConfigError::ConversionError {
        key,
        source_index,
        source,
    } = error
    else {
        panic!("expected a conversion error");
    };
    assert_eq!(key, "x.val");
    assert_eq!(source_index, Some(0));
    assert_eq!(
        source,
        DataConversionError::invalid(
            DataType::Float32,
            DataType::Json,
            InvalidValueReason::NonFinite,
        )
    );
}

#[test]
fn test_deserialize_float64_infinity_is_rejected_with_source_index() {
    let config = config_with_mv("x.val", MultiValues::Float64(vec![1.0, f64::INFINITY]));
    let error = config.deserialize::<AnyStruct>("x").unwrap_err();

    let ConfigError::ConversionError {
        key,
        source_index,
        source,
    } = error
    else {
        panic!("expected a conversion error");
    };
    assert_eq!(key, "x.val");
    assert_eq!(source_index, Some(1));
    assert_eq!(
        source,
        DataConversionError::invalid(
            DataType::Float64,
            DataType::Json,
            InvalidValueReason::NonFinite,
        )
    );
}

#[test]
fn test_deserialize_duration() {
    let config = config_with_value("x.val", Value::Duration(Duration::from_millis(500)));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!("500ms"));
}

#[test]
fn test_deserialize_duration_rejects_implicit_precision_loss() {
    let config = config_with_value("x.val", Value::Duration(Duration::from_micros(1500)));
    let error = config.deserialize::<AnyStruct>("x").unwrap_err();

    assert!(matches!(
        error,
        ConfigError::ConversionError { source, .. }
            if source.reason() == Some(&InvalidValueReason::PrecisionLoss)
    ));
}

#[test]
fn test_deserialize_duration_allows_explicit_half_up_rounding() {
    let mut config = config_with_value("x.val", Value::Duration(Duration::from_micros(1500)));
    config.set_default_read_policy(ReadPolicy::default().with_duration_options(
        DurationConversionOptions::default().with_rounding_policy(DurationRoundingPolicy::HalfUp),
    ));

    let value: AnyStruct = config.deserialize("x").unwrap();

    assert_eq!(value.val, serde_json::json!("2ms"));
}

#[test]
fn test_deserialize_duration_honors_output_unit() {
    let mut config = config_with_value("x.val", Value::Duration(Duration::from_micros(1500)));
    config.set_default_read_policy(ReadPolicy::default().with_duration_options(
        DurationConversionOptions::default().with_output_unit(DurationUnit::Microseconds),
    ));

    let value: AnyStruct = config.deserialize("x").unwrap();

    assert_eq!(value.val, serde_json::json!("1500µs"));
}

#[cfg(feature = "url")]
#[test]
fn test_deserialize_url() {
    let url = Url::parse("https://example.com").unwrap();
    let config = config_with_value("x.val", Value::new(url));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.as_str().unwrap().contains("example.com"));
}

#[test]
fn test_deserialize_string_map_single() {
    let mut map = std::collections::HashMap::new();
    map.insert("key".to_string(), "value".to_string());
    let config = config_with_value("x.val", Value::StringMap(map));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_object());
    assert_eq!(s.val["key"], serde_json::json!("value"));
}

#[test]
fn test_deserialize_string_map_multi() {
    let mut map1 = std::collections::HashMap::new();
    map1.insert("k1".to_string(), "v1".to_string());
    let mut map2 = std::collections::HashMap::new();
    map2.insert("k2".to_string(), "v2".to_string());
    let config = config_with_mv("x.val", MultiValues::StringMap(vec![map1, map2]));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_array());
}

#[test]
fn test_deserialize_json_single() {
    let json_val = serde_json::json!({"nested": true});
    let config = config_with_value("x.val", Value::Json(json_val.clone()));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, json_val);
}

#[test]
fn test_deserialize_json_multi() {
    let j1 = serde_json::json!(1);
    let j2 = serde_json::json!(2);
    let config = config_with_mv("x.val", MultiValues::Json(vec![j1, j2]));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_array());
}

#[test]
fn test_deserialize_char() {
    let config = config_with_value("x.val", Value::Char('A'));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!("A"));
}

#[cfg(feature = "num-bigint")]
#[test]
fn test_deserialize_big_integer() {
    let big = BigInt::from(12345678901234567i64);
    let config = config_with_value("x.val", Value::BigInteger(big));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[cfg(feature = "bigdecimal")]
#[test]
fn test_deserialize_big_decimal() {
    let dec = BigDecimal::from_str("3.14159265358979").unwrap();
    let config = config_with_value("x.val", Value::BigDecimal(dec));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[cfg(feature = "chrono")]
#[test]
fn test_deserialize_datetime() {
    let dt = NaiveDateTime::parse_from_str("2026-04-09 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let config = config_with_value("x.val", Value::DateTime(dt));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[cfg(feature = "chrono")]
#[test]
fn test_deserialize_date() {
    let d = NaiveDate::from_ymd_opt(2026, 4, 9).unwrap();
    let config = config_with_value("x.val", Value::Date(d));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[cfg(feature = "chrono")]
#[test]
fn test_deserialize_time() {
    let t = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
    let config = config_with_value("x.val", Value::Time(t));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[cfg(feature = "chrono")]
#[test]
fn test_deserialize_instant() {
    let instant: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-04-09T12:00:00Z")
        .unwrap()
        .into();
    let config = config_with_value("x.val", Value::Instant(instant));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[test]
fn test_deserialize_int128() {
    let config = config_with_value("x.val", Value::Int128(42i128));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[test]
fn test_deserialize_uint128() {
    let config = config_with_value("x.val", Value::UInt128(42u128));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_string());
}

#[test]
fn test_deserialize_unset_multivalue_is_null() {
    let config = config_with_mv("x.val", MultiValues::Unset(DataType::String));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_null());
}

#[test]
fn test_deserialize_empty_string_multivalue_is_array() {
    let config = config_with_mv("x.val", MultiValues::String(Vec::new()));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert_eq!(s.val, serde_json::json!([]));
}

#[test]
fn test_deserialize_multi_int32_array() {
    let config = config_with_mv("x.val", MultiValues::Int32(vec![1, 2, 3]));
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_array());
    assert_eq!(s.val.as_array().unwrap().len(), 3);
}

#[test]
fn test_deserialize_multi_string_array() {
    let config = config_with_mv(
        "x.val",
        MultiValues::String(vec!["a".to_string(), "b".to_string()]),
    );
    let s: AnyStruct = config.deserialize("x").unwrap();
    assert!(s.val.is_array());
}
