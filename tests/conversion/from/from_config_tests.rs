// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for typed `FromConfig` parsing behavior.

#[cfg(any(feature = "bigdecimal", feature = "num-bigint"))]
use std::str::FromStr;
use std::time::Duration;

#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
#[cfg(feature = "chrono")]
use chrono::NaiveDate;
#[cfg(feature = "chrono")]
use chrono::NaiveDateTime;
#[cfg(feature = "num-bigint")]
use num_bigint::BigInt;
use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::Property;
use qubit_config::conversion::ConfigParseContext;
use qubit_config::conversion::FromConfig;
use qubit_config::options::ReadPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
use qubit_datatype::NumericConversionPolicy;

#[derive(Debug, PartialEq)]
struct ContextAware(String);

impl FromConfig for ContextAware {
    fn from_config(_property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let value = if ctx.interpolates() {
            ctx.substitute_string("${base}")?
        } else {
            "${base}".to_string()
        };
        Ok(Self(value))
    }
}

#[test]
fn test_external_from_config_can_observe_interpolation_context() {
    let mut config = Config::new();
    config.set("base", "resolved").expect("set base");
    config.set("value", "ignored").expect("set value");

    assert_eq!(config.get::<ContextAware>("value").unwrap().0, "${base}");
    assert_eq!(config.get_interpolated::<ContextAware>("value").unwrap().0, "resolved");
}

#[test]
fn test_external_from_config_preserves_substitution_errors() {
    let mut config = Config::new();
    config.set("value", "ignored").expect("set value");
    config.set("base", "${base}").expect("set cyclic base");

    let error = config
        .get_interpolated::<ContextAware>("value")
        .expect_err("cyclic substitution should fail");
    assert!(matches!(error, ConfigError::SubstitutionCycle { .. }));
}

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

#[test]
fn test_usize_conversion_checks_platform_range_and_preserves_context() {
    let mut config = Config::new();
    config.set("server.zero", 0_u128).expect("set zero");
    config.set("server.maximum", usize::MAX as u128).expect("set maximum");
    config
        .set("server.limit", usize::MAX as u128 + 1)
        .expect("set out-of-range value");
    config
        .set("server.items", vec![usize::MAX as u128 + 1, 0_u128])
        .expect("set out-of-range collection");
    let server = config.section("server").expect("server section");

    assert_eq!(server.get::<usize>("zero").expect("zero fits"), 0);
    assert_eq!(server.get::<usize>("maximum").expect("maximum fits"), usize::MAX);

    let error = server.get::<usize>("limit").expect_err("overflow must fail");
    assert!(matches!(error, ConfigError::PlatformIntegerOutOfRange { .. }));
    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.limit"));
    assert_eq!(error.source_index(), None);

    let error = server.get::<usize>("items").expect_err("first item overflow must fail");
    assert!(matches!(error, ConfigError::PlatformIntegerOutOfRange { .. }));
    assert_eq!(error.path(), Some("server.items"));
    assert_eq!(error.source_index(), Some(0));
}

#[test]
fn test_usize_conversion_supports_optional_default_and_interpolation() {
    let mut config = Config::new();
    config.set("fallback", "42").expect("set fallback");
    config.set("resolved", "17").expect("set resolved");
    config.set("limit", "${resolved}").expect("set interpolated value");

    assert_eq!(config.get_optional::<usize>("missing").expect("optional read"), None);
    assert_eq!(config.get_or("missing", 29_usize).expect("default read"), 29);
    assert_eq!(config.get::<usize>("fallback").expect("parse string"), 42);
    assert_eq!(config.get_interpolated::<usize>("limit").expect("interpolate"), 17);

    config.set("negative", -1_i64).expect("set negative");
    assert!(matches!(
        config.get::<usize>("negative"),
        Err(ConfigError::ConversionError { .. })
    ));
}

/// Test typed vectors are converted without a lossy string intermediary.
#[test]
fn test_from_config_preserves_typed_vector_values() {
    let durations = vec![Duration::new(1, 1), Duration::new(2, 999_999_999)];
    let chars = vec!['A', '中'];
    let mut config = Config::new();
    config.set("chars", chars.clone()).expect("set chars");
    config.set("durations", durations.clone()).expect("set durations");

    assert_eq!(config.get::<Vec<char>>("chars").unwrap(), chars);
    assert_eq!(config.get::<Vec<Duration>>("durations").unwrap(), durations,);
}

/// Test chrono vectors preserve their original representations.
#[cfg(feature = "chrono")]
#[test]
fn test_from_config_preserves_chrono_vector_values() {
    let dates = vec![NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()];
    let datetimes =
        vec![NaiveDateTime::parse_from_str("2026-07-13T01:02:03.123456789", "%Y-%m-%dT%H:%M:%S%.f").unwrap()];
    let mut config = Config::new();
    config.set("dates", dates.clone()).unwrap();
    config.set("datetimes", datetimes.clone()).unwrap();

    assert_eq!(config.get::<Vec<NaiveDate>>("dates").unwrap(), dates);
    assert_eq!(config.get::<Vec<NaiveDateTime>>("datetimes").unwrap(), datetimes,);
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
    config.set("ports", vec!["1", "bad", "3"]).expect("set ports");

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
        ReadPolicy::builder()
            .numeric_policy(NumericConversionPolicy::lossy())
            .build(),
    );
    assert_eq!(config.get::<Vec<i32>>("values").unwrap(), vec![1, 2]);
}

#[test]
fn test_from_config_interpolation_preserves_non_string_shapes() {
    let mut config = Config::new();
    config.set("base", "41").unwrap();
    config.set("scalar", 7_u16).unwrap();
    config.set("numbers", vec![1_u16, 2_u16]).unwrap();
    config.set("interpolated", "${base}").unwrap();
    config.set("bad_base", "not-a-number").unwrap();
    config.set("bad_interpolated", "${bad_base}").unwrap();
    config.set("invalid", "${missing}").unwrap();

    assert_eq!(config.get_interpolated::<Vec<u16>>("scalar").unwrap(), vec![7]);
    assert_eq!(config.get_interpolated::<Vec<u16>>("numbers").unwrap(), vec![1, 2]);
    assert_eq!(config.get_interpolated::<u16>("interpolated").unwrap(), 41);
    assert!(matches!(
        config.get_interpolated::<u16>("bad_interpolated"),
        Err(ConfigError::ConversionError { key, .. }) if key == "bad_interpolated"
    ));
    assert!(matches!(
        config.get_interpolated::<u16>("invalid"),
        Err(ConfigError::SubstitutionError { path, .. }) if path == "invalid"
    ));
}

#[test]
fn test_from_config_interpolates_string_scalar_and_collection_values() {
    let mut config = Config::new();
    config.set("base", "resolved").unwrap();
    config.set("scalar", "prefix-${base}").unwrap();
    config.set("values", vec!["${base}", "suffix-${base}"]).unwrap();

    assert_eq!(config.get::<String>("scalar").unwrap(), "prefix-${base}");
    assert_eq!(config.get_interpolated::<String>("scalar").unwrap(), "prefix-resolved",);
    assert_eq!(
        config.get_interpolated::<Vec<String>>("values").unwrap(),
        ["resolved", "suffix-resolved"],
    );
}

#[test]
fn test_interpolated_scalar_converts_only_the_first_string_collection_item() {
    let mut config = Config::new();
    config.set("base", "8080").unwrap();
    config.set("ports", vec!["${base}", "${missing}"]).unwrap();

    assert_eq!(config.get::<String>("ports").unwrap(), "${base}");
    assert_eq!(config.get_interpolated::<String>("ports").unwrap(), "8080");
    assert_eq!(config.get_interpolated::<u16>("ports").unwrap(), 8080);
    assert!(config.get_interpolated::<Vec<u16>>("ports").is_err());
}

#[test]
fn test_interpolated_scalar_keeps_empty_unset_and_non_string_paths() {
    let mut config = Config::new();
    config.set("empty", Vec::<String>::new()).unwrap();
    config.set_null("unset", DataType::String).unwrap();
    config.set("numbers", vec![7_u16, 8_u16]).unwrap();

    assert!(config.get::<u16>("empty").is_err());
    assert!(config.get_interpolated::<u16>("empty").is_err());
    assert!(config.get::<String>("unset").is_err());
    assert!(config.get_interpolated::<String>("unset").is_err());
    assert_eq!(config.get::<u16>("numbers").unwrap(), 7);
    assert_eq!(config.get_interpolated::<u16>("numbers").unwrap(), 7);
}

#[test]
fn test_interpolated_string_output_limit_preserves_property_path() {
    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_output_bytes(1).build())
        .build();
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().conversion_limits(limits).build())
        .build();
    config.set("base", "long").unwrap();
    config.set("service.name", "${base}").unwrap();

    let error = config.get_interpolated::<String>("service.name").unwrap_err();
    assert_eq!(error.path(), Some("base"));
}

#[derive(Debug, PartialEq)]
struct ReplacementNumber(u16);

impl FromConfig for ReplacementNumber {
    fn from_config(_property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let replacement = Property::new("replacement", "20").expect("valid replacement property");
        u16::from_config(&replacement, ctx).map(Self)
    }
}

#[derive(Debug, PartialEq)]
struct ReplacementCollection(u16);

impl FromConfig for ReplacementCollection {
    fn from_config(_property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let replacement = Property::new("replacement", vec!["20", "30"]).expect("valid replacement collection");
        u16::from_config(&replacement, ctx).map(Self)
    }
}

#[derive(Debug, PartialEq)]
struct ReplacementString(String);

impl FromConfig for ReplacementString {
    fn from_config(_property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let replacement = Property::new("replacement", "${base}-new").expect("valid replacement property");
        String::from_config(&replacement, ctx).map(Self)
    }
}

#[derive(Debug, PartialEq)]
struct ReplacementStringCollection(String);

impl FromConfig for ReplacementStringCollection {
    fn from_config(_property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let replacement =
            Property::new("replacement", vec!["${base}", "${missing}"]).expect("valid replacement collection");
        String::from_config(&replacement, ctx).map(Self)
    }
}

#[test]
fn test_delegated_scalar_conversion_uses_replacement_property_text() {
    let mut config = Config::new();
    config.set("base", "10").unwrap();
    config.set("value", "${base}").unwrap();

    assert_eq!(
        config.get_interpolated::<ReplacementNumber>("value").unwrap(),
        ReplacementNumber(20)
    );
}

#[test]
fn test_delegated_collection_conversion_does_not_use_scalar_cache() {
    let mut config = Config::new();
    config.set("base", "10").unwrap();
    config.set("value", vec!["${base}"]).unwrap();

    assert_eq!(
        config.get_interpolated::<ReplacementNumber>("value").unwrap(),
        ReplacementNumber(20)
    );
}

#[test]
fn test_delegated_scalar_conversion_accepts_replacement_collection() {
    let mut config = Config::new();
    config.set("value", "10").unwrap();

    assert_eq!(
        config.get_interpolated::<ReplacementCollection>("value").unwrap(),
        ReplacementCollection(20)
    );
}

#[test]
fn test_delegated_string_conversion_uses_replacement_property_text() {
    let mut config = Config::new();
    config.set("http.base", "resolved").unwrap();
    config.set("http.value", "${base}").unwrap();
    let section = config.section("http").unwrap();

    assert_eq!(
        section.get_interpolated::<ReplacementString>("value").unwrap(),
        ReplacementString("resolved-new".to_owned()),
    );
}

#[test]
fn test_delegated_string_conversion_interpolates_replacement_collection_first_item() {
    let mut config = Config::new();
    config.set("base", "resolved").unwrap();
    config.set("value", "ignored").unwrap();

    assert_eq!(
        config.get_interpolated::<ReplacementStringCollection>("value").unwrap(),
        ReplacementStringCollection("resolved".to_owned())
    );
}
