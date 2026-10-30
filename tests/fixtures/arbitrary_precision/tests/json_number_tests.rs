// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ReadPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct NestedNumber {
    value: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StringPair {
    a: String,
    b: String,
}

fn config_with_json(key: &str, text: &str) -> Config {
    let value: serde_json::Value = serde_json::from_str(text).expect("the JSON number should parse");
    let mut config = Config::new();
    config.set(key, value).expect("the JSON value should be set");
    config
}

#[test]
fn test_json_wide_integer_keeps_exact_value() {
    let config = config_with_json("number", "18446744073709551617");
    assert_eq!(
        config.deserialize::<u128>("number").expect("the value fits u128"),
        18_446_744_073_709_551_617_u128,
    );
}

#[test]
fn test_json_nonfinite_conversion_returns_error() {
    let config = config_with_json("number", "1e400");
    let error = config
        .deserialize::<f64>("number")
        .expect_err("infinity must be rejected");
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("number"));
    assert!(!error.to_string().contains("1e400"));

    let integer_error = config
        .deserialize::<u128>("number")
        .expect_err("non-finite JSON numbers must not be converted to integers");
    assert_eq!(integer_error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(integer_error.path(), Some("number"));
    assert!(!integer_error.to_string().contains("1e400"));
}

#[test]
fn test_json_128_bit_boundaries_are_exact() {
    let signed = config_with_json("number", "-170141183460469231731687303715884105728");
    assert_eq!(signed.deserialize::<i128>("number").unwrap(), i128::MIN);

    let unsigned = config_with_json("number", "340282366920938463463374607431768211455");
    assert_eq!(unsigned.deserialize::<u128>("number").unwrap(), u128::MAX);
}

#[test]
fn test_json_integers_outside_128_bit_ranges_are_rejected() {
    for (text, signed) in [
        ("-170141183460469231731687303715884105729", true),
        ("340282366920938463463374607431768211456", false),
    ] {
        let config = config_with_json("number", text);
        let error = if signed {
            config.deserialize::<i128>("number").unwrap_err()
        } else {
            config.deserialize::<u128>("number").unwrap_err()
        };
        assert_eq!(error.kind(), ConfigErrorKind::Conversion);
        assert_eq!(error.path(), Some("number"));
        assert!(!error.to_string().contains(text));
    }
}

#[test]
fn test_json_64_bit_boundaries_and_finite_float_are_preserved() {
    let signed = config_with_json("number", "-9223372036854775808");
    assert_eq!(signed.deserialize::<i64>("number").unwrap(), i64::MIN);

    let unsigned = config_with_json("number", "18446744073709551615");
    assert_eq!(unsigned.deserialize::<u64>("number").unwrap(), u64::MAX);

    let float = config_with_json("number", "1.25e2");
    assert_eq!(float.deserialize::<f64>("number").unwrap(), 125.0);
}

#[test]
fn test_json_natural_projection_uses_decimal_strings_for_wide_integers() {
    let config = config_with_json("number", "18446744073709551617");
    assert_eq!(
        config.deserialize::<serde_json::Value>("number").unwrap(),
        serde_json::Value::String("18446744073709551617".to_owned()),
    );
}

#[test]
fn test_json_wide_natural_projection_obeys_output_budget() {
    for text in ["18446744073709551617", "-9223372036854775809"] {
        for maximum in [0_u64, 19, 20] {
            let limits = ConversionLimits::builder()
                .operation_limits(
                    ConversionOperationLimits::builder()
                        .max_output_bytes(maximum)
                        .build(),
                )
                .build();
            let mut config = config_with_json("number", text);
            config.set_default_read_policy(
                ReadPolicy::builder().conversion_limits(limits).build(),
            );
            let result = config.deserialize::<serde_json::Value>("number");
            if maximum < 20 {
                let error = result.expect_err("wide text must consume output bytes");
                assert_eq!(error.kind(), ConfigErrorKind::Conversion);
                assert_eq!(error.path(), Some("number"));
                assert!(error.to_string().contains("OutputBytes"));
                assert!(!error.to_string().contains(text));
            } else {
                assert_eq!(result.unwrap(), serde_json::Value::String(text.to_owned()));
            }
        }
    }

    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_output_bytes(19)
                .build(),
        )
        .build();
    let mut unsigned = Config::new();
    unsigned
        .set("number", 18_446_744_073_709_551_617_u128)
        .expect("native unsigned integer should be set");
    unsigned.set_default_read_policy(
        ReadPolicy::builder().conversion_limits(limits.clone()).build(),
    );
    let error = unsigned
        .deserialize::<serde_json::Value>("number")
        .expect_err("native unsigned wide text must consume output bytes");
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("number"));
    assert!(error.to_string().contains("OutputBytes"));

    let mut signed = Config::new();
    signed
        .set("number", -9_223_372_036_854_775_809_i128)
        .expect("native signed integer should be set");
    signed.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
    let error = signed
        .deserialize::<serde_json::Value>("number")
        .expect_err("native signed wide text must consume output bytes");
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("number"));
    assert!(error.to_string().contains("OutputBytes"));

    let zero_output = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_output_bytes(0)
                .build(),
        )
        .build();
    let mut typed_unsigned = config_with_json("number", "18446744073709551617");
    typed_unsigned.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_limits(zero_output.clone())
            .build(),
    );
    assert_eq!(
        typed_unsigned.deserialize::<u128>("number").unwrap(),
        18_446_744_073_709_551_617_u128
    );

    let mut typed_signed = config_with_json("number", "-9223372036854775809");
    typed_signed.set_default_read_policy(ReadPolicy::builder().conversion_limits(zero_output).build());
    assert_eq!(
        typed_signed.deserialize::<i128>("number").unwrap(),
        -9_223_372_036_854_775_809_i128
    );
}

#[test]
fn test_json_wide_natural_projection_shares_output_budget() {
    let text = "[18446744073709551617,18446744073709551617]";
    for maximum in [39_u64, 40] {
        let limits = ConversionLimits::builder()
            .operation_limits(
                ConversionOperationLimits::builder()
                    .max_output_bytes(maximum)
                    .build(),
            )
            .build();
        let mut config = config_with_json("numbers", text);
        config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
        let result = config.deserialize::<serde_json::Value>("numbers");
        if maximum == 39 {
            let error = result.expect_err("the second string must exhaust the shared budget");
            assert_eq!(error.kind(), ConfigErrorKind::Conversion);
            assert_eq!(error.path(), Some("numbers[1]"));
            assert_eq!(error.source_index(), Some(1));
            assert!(error.to_string().contains("OutputBytes"));
        } else {
            assert_eq!(
                result.unwrap(),
                serde_json::json!(["18446744073709551617", "18446744073709551617"])
            );
        }
    }
}

#[test]
fn test_json_nested_and_array_errors_keep_path_and_source_index() {
    let nested = config_with_json("nested", r#"{"value":1e400}"#);
    let nested_error = nested.deserialize::<NestedNumber>("nested").unwrap_err();
    assert_eq!(nested_error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(nested_error.path(), Some("nested.value"));

    let list = config_with_json("values", "[1.0,1e400]");
    let list_error = list.deserialize::<Vec<f64>>("values").unwrap_err();
    assert_eq!(list_error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(list_error.path(), Some("values[1]"));
    assert_eq!(list_error.source_index(), Some(1));
}

#[test]
fn test_two_field_materialization_shares_cumulative_conversion_budget() {
    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_input_bytes(3).build())
        .build();
    let policy = ReadPolicy::builder().conversion_limits(limits).build();
    let mut config = Config::new();
    config
        .set_default_read_policy(policy)
        .set("a", "aa")
        .expect("first field should be stored");
    config.set("b", "bb").expect("second field should be stored");

    let error = config
        .deserialize::<StringPair>("")
        .expect_err("the second field should exceed the shared input-byte budget");
    assert_eq!(error.kind(), ConfigErrorKind::Value);
    assert_eq!(error.path(), Some("b"), "{error}");
    assert!(error.to_string().contains("InputBytes"));
}
