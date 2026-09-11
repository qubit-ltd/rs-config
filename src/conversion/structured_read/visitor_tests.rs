// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Direct source visitors must preserve types, source positions, and budgets.

use qubit_datatype::BlankStringPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_datatype::DataConverter;
use qubit_datatype::DataType;
use qubit_value::ValueContainer;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error;
use serde::de::MapAccess;
use serde::de::Visitor;

use super::deserialize_from_reader;
use crate::Config;
use crate::ConfigError;
use crate::ConfigReader;
use crate::ReadPolicy;

#[test]
fn wide_numbers_remain_exact_and_any_uses_natural_text() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wide {
        low: i128,
        high: u128,
    }
    let mut config = Config::new();
    config.set("wide.low", i128::MIN).unwrap();
    config.set("wide.high", u128::MAX).unwrap();
    let actual: Wide = deserialize_from_reader(&config, "wide", false, true).unwrap();
    assert_eq!(
        actual,
        Wide {
            low: i128::MIN,
            high: u128::MAX
        }
    );
    let actual: serde_json::Value = deserialize_from_reader(&config, "wide", false, true).unwrap();
    assert_eq!(
        actual,
        serde_json::json!({"low": i128::MIN.to_string(), "high": u128::MAX.to_string()})
    );
}

#[test]
fn typed_numeric_errors_retain_the_original_source_type() {
    let mut config = Config::new();
    config.set("number", 1.5_f64).unwrap();
    let expected = DataConverter::from(1.5_f64)
        .to_with::<u8>(
            config.read_policy().conversion_policy(),
            config.read_policy().conversion_limits(),
        )
        .unwrap_err();
    let error = deserialize_from_reader::<_, u8>(&config, "number", false, true).unwrap_err();
    assert!(matches!(error, ConfigError::ConversionError { key, source, .. } if key == "number" && source == expected));
}

#[test]
fn float32_any_keeps_natural_precision() {
    let mut config = Config::new();
    config.set("number", 0.1_f32).unwrap();
    let typed: f32 = deserialize_from_reader(&config, "number", false, true).unwrap();
    assert_eq!(typed.to_bits(), 0.1_f32.to_bits());
    let any: serde_json::Value = deserialize_from_reader(&config, "number", false, true).unwrap();
    assert_eq!(any, serde_json::json!(0.1));
}

#[test]
fn options_keep_unset_null_and_empty_collections_distinct() {
    let mut config = Config::new();
    config.set_null("unset", DataType::Int32).unwrap();
    config.set("null", serde_json::Value::Null).unwrap();
    config.set("empty", Vec::<i32>::new()).unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Option<i32>>(&config, "unset", false, true).unwrap(),
        None
    );
    assert_eq!(
        deserialize_from_reader::<_, Option<i32>>(&config, "null", false, true).unwrap(),
        None
    );
    assert_eq!(
        deserialize_from_reader::<_, Option<Vec<i32>>>(&config, "empty", false, true).unwrap(),
        Some(vec![])
    );
    let error = deserialize_from_reader::<_, i32>(&config, "unset", false, true).unwrap_err();
    assert!(error.value_missing().unwrap().is_unset());
    assert_eq!(error.value_missing().unwrap().target_type(), Some(DataType::Int32));
}

#[test]
fn scalar_split_has_one_admission_and_collection_strings_never_split_again() {
    let mut config = Config::new();
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_items(2)
                .max_input_bytes(10)
                .max_output_bytes(0)
                .build(),
        )
        .build();
    config.set_default_read_policy(
        ReadPolicy::builder_from(&ReadPolicy::env_friendly())
            .conversion_limits(limits)
            .build(),
    );
    config.set("ports", "8080, 8081").unwrap();
    let values: Vec<u16> = deserialize_from_reader(&config, "ports", false, true).unwrap();
    assert_eq!(values, [8080, 8081]);
    config.set("ports", vec!["1,2"]).unwrap();
    assert!(deserialize_from_reader::<_, Vec<Vec<u16>>>(&config, "ports", false, true).is_err());
}

#[test]
fn all_numeric_and_any_leaves_share_one_session() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Pair {
        first: i32,
        second: i32,
    }
    let mut config = Config::new();
    config.set("first", 1_i32).unwrap();
    config.set("second", 2_i32).unwrap();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_limits(
                ConversionLimits::builder()
                    .operation_limits(ConversionOperationLimits::builder().max_items(1).build())
                    .build(),
            )
            .build(),
    );
    assert!(
        deserialize_from_reader::<_, Pair>(&config, "", false, true)
            .unwrap_err()
            .to_string()
            .contains("Items")
    );
    assert!(
        deserialize_from_reader::<_, serde_json::Value>(&config, "", false, true)
            .unwrap_err()
            .to_string()
            .contains("Items")
    );
}

#[test]
fn nested_newtypes_do_not_repeat_leaf_admission() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Inner(String);
    #[derive(Debug, Deserialize, PartialEq)]
    struct Outer(Inner);
    let mut config = Config::new();
    config.set("text", "ab").unwrap();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_limits(
                ConversionLimits::builder()
                    .operation_limits(
                        ConversionOperationLimits::builder()
                            .max_items(1)
                            .max_input_bytes(2)
                            .max_output_bytes(2)
                            .build(),
                    )
                    .build(),
            )
            .build(),
    );
    assert_eq!(
        deserialize_from_reader::<_, Outer>(&config, "text", false, true).unwrap(),
        Outer(Inner("ab".into()))
    );
}

#[test]
fn external_enum_variants_and_nested_paths_are_preserved() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Mode {
        Off,
        Count(u16),
        Pair(u16, String),
        Named { port: u16 },
    }
    let mut config = Config::new();
    for (value, expected) in [
        (serde_json::json!("Off"), Mode::Off),
        (serde_json::json!({"Count": "3"}), Mode::Count(3)),
        (serde_json::json!({"Pair": [4, "text"]}), Mode::Pair(4, "text".into())),
        (
            serde_json::json!({"Named": {"port": "8080"}}),
            Mode::Named { port: 8080 },
        ),
    ] {
        config.set("mode", value).unwrap();
        assert_eq!(
            deserialize_from_reader::<_, Mode>(&config, "mode", false, true).unwrap(),
            expected
        );
    }
    config
        .set("mode", serde_json::json!({"Named": {"port": "secret-invalid"}}))
        .unwrap();
    let error = deserialize_from_reader::<_, Mode>(&config, "mode", false, true).unwrap_err();
    assert_eq!(error.path(), Some("mode.Named.port"));
    assert!(!error.to_string().contains("secret-invalid"));
}

#[test]
fn ignored_fields_are_reported_or_admitted_without_consumption() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Known {
        port: u16,
    }
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.ignored", "x".repeat(65536)).unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Known>(&config, "server", false, false).unwrap(),
        Known { port: 8080 }
    );
    let error = deserialize_from_reader::<_, Known>(&config, "server", false, true).unwrap_err();
    assert_eq!(error.unknown_property_paths().unwrap(), ["server.ignored"]);
    config.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_limits(
                ConversionLimits::builder()
                    .operation_limits(
                        ConversionOperationLimits::builder()
                            .max_structured_payload_bytes(10)
                            .build(),
                    )
                    .build(),
            )
            .build(),
    );
    assert!(deserialize_from_reader::<_, Known>(&config, "server", false, false).is_err());
}

#[test]
fn collection_and_split_failures_keep_original_indices() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    for value in [
        ValueContainer::from("1, secret-invalid, 3"),
        ValueContainer::from(vec!["1", "secret-invalid", "3"]),
    ] {
        config.set("values", value).unwrap();
        let error = deserialize_from_reader::<_, Vec<u16>>(&config, "values", false, true).unwrap_err();
        assert_eq!(error.path(), Some("values[1]"));
        assert_eq!(error.source_index(), Some(1));
        assert!(!error.to_string().contains("secret-invalid"));
    }
}

#[test]
fn tuple_lengths_and_collection_missing_are_not_silently_ignored() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    for value in [ValueContainer::from(vec![1_i32, 2, 3]), ValueContainer::from("1,2,3")] {
        config.set("values", value).unwrap();
        assert!(deserialize_from_reader::<_, (i32, i32)>(&config, "values", false, true).is_err());
    }
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::TreatAsMissing)
            .build(),
    );
    config.set("values", vec!["1", " "]).unwrap();
    let error = deserialize_from_reader::<_, Vec<Option<i32>>>(&config, "values", false, true).unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert!(!error.value_missing().unwrap().is_defaultable_for_conversion());
}

#[test]
fn interpolation_uses_overlays_for_nested_maps_sequences_and_missing_options() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Settings {
        ports: Vec<u16>,
        title: Option<String>,
    }
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::TreatAsMissing)
            .build(),
    );
    config.set("port", "8080").unwrap();
    config.set("blank", " ").unwrap();
    config.set("settings.ports", vec!["${port}"]).unwrap();
    config.set("settings.title", "${blank}").unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Settings>(&config, "settings", true, true).unwrap(),
        Settings {
            ports: vec![8080],
            title: None
        }
    );
    config
        .set("json", serde_json::json!({"ports": ["${port}"], "title": "hello"}))
        .unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Settings>(&config, "json", true, true).unwrap(),
        Settings {
            ports: vec![8080],
            title: Some("hello".into())
        }
    );
}

#[test]
fn standard_duration_requires_its_actual_serde_structure() {
    let mut config = Config::new();
    config.set("duration", std::time::Duration::from_secs(2)).unwrap();
    assert!(deserialize_from_reader::<_, std::time::Duration>(&config, "duration", false, true).is_err());
    config
        .set("duration", serde_json::json!({"secs": 2, "nanos": 3}))
        .unwrap();
    assert_eq!(
        deserialize_from_reader::<_, std::time::Duration>(&config, "duration", false, true).unwrap(),
        std::time::Duration::new(2, 3)
    );
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_targets_receive_their_requested_string_representation() {
    let mut config = Config::new();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    config.set("date", date).unwrap();
    assert_eq!(
        deserialize_from_reader::<_, chrono::NaiveDate>(&config, "date", false, true).unwrap(),
        date
    );
}

#[cfg(feature = "bigdecimal")]
#[test]
fn bigdecimal_target_accepts_the_natural_rich_text() {
    let mut config = Config::new();
    let value: bigdecimal::BigDecimal = "1234567890.123456789".parse().unwrap();
    config.set("decimal", value.clone()).unwrap();
    assert_eq!(
        deserialize_from_reader::<_, bigdecimal::BigDecimal>(&config, "decimal", false, true).unwrap(),
        value
    );
}

#[test]
fn byte_visitors_receive_utf8_from_string_compatible_scalars() {
    #[derive(Debug, PartialEq)]
    struct Bytes(Vec<u8>);
    impl<'de> Deserialize<'de> for Bytes {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct ByteVisitor;
            impl<'de> Visitor<'de> for ByteVisitor {
                type Value = Bytes;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("bytes")
                }
                fn visit_byte_buf<E: Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                    Ok(Bytes(value))
                }
            }
            deserializer.deserialize_bytes(ByteVisitor)
        }
    }
    let mut config = Config::new();
    config.set("text", "中文").unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Bytes>(&config, "text", false, true).unwrap(),
        Bytes("中文".as_bytes().to_vec())
    );
}

#[test]
fn map_value_requested_before_key_returns_an_error() {
    #[derive(Debug)]
    struct WrongOrder;
    impl<'de> Deserialize<'de> for WrongOrder {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct WrongVisitor;
            impl<'de> Visitor<'de> for WrongVisitor {
                type Value = WrongOrder;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("map")
                }
                fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                    let _: String = access.next_value()?;
                    Ok(WrongOrder)
                }
            }
            deserializer.deserialize_map(WrongVisitor)
        }
    }
    let mut config = Config::new();
    config.set("tree.key", "value").unwrap();
    for reject_unknown in [false, true] {
        let error = deserialize_from_reader::<_, WrongOrder>(&config, "tree", false, reject_unknown).unwrap_err();
        assert_eq!(error.path(), Some("tree"));
    }
}

#[test]
fn invalid_map_keys_and_enum_payloads_preserve_paths() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Mode {
        Off,
        Count(u16),
        Pair(u16, String),
    }
    let mut config = Config::new();
    config.set("tree.key", "value").unwrap();
    let error = deserialize_from_reader::<_, std::collections::BTreeMap<u32, String>>(&config, "tree", false, false)
        .unwrap_err();
    assert_eq!(error.path(), Some("tree"));
    for (value, path) in [
        (serde_json::json!({"Unknown": null}), "mode.Unknown"),
        (serde_json::json!({"Count": "secret-invalid"}), "mode.Count"),
        (serde_json::json!({"Pair": ["secret-invalid", "text"]}), "mode.Pair[0]"),
    ] {
        config.set("mode", value).unwrap();
        let error = deserialize_from_reader::<_, Mode>(&config, "mode", false, false).unwrap_err();
        assert_eq!(error.path(), Some(path));
        assert!(!error.to_string().contains("secret-invalid"));
    }
    config.set("mode", serde_json::json!({"Off": null})).unwrap();
    assert_eq!(
        deserialize_from_reader::<_, Mode>(&config, "mode", false, false).unwrap(),
        Mode::Off
    );
}

#[test]
fn scalar_sequence_source_and_unconsumed_tail_limits_preserve_errors() {
    use qubit_datatype::CollectionConversionLimits;
    let mut config = Config::new();
    config.set("ports", "1,2,3").unwrap();
    config.set_default_read_policy(
        ReadPolicy::builder_from(&ReadPolicy::env_friendly())
            .conversion_limits(
                ConversionLimits::builder()
                    .collection_limits(CollectionConversionLimits::builder().max_source_bytes(4).build())
                    .build(),
            )
            .build(),
    );
    let error = deserialize_from_reader::<_, Vec<u16>>(&config, "ports", false, false).unwrap_err();
    assert_eq!(error.path(), Some("ports"));
    assert!(error.to_string().contains("CollectionSourceBytes"));
    config.set_default_read_policy(
        ReadPolicy::builder_from(&ReadPolicy::env_friendly())
            .conversion_limits(
                ConversionLimits::builder()
                    .operation_limits(ConversionOperationLimits::builder().max_items(2).build())
                    .build(),
            )
            .build(),
    );
    let error = deserialize_from_reader::<_, (u16, u16)>(&config, "ports", false, false).unwrap_err();
    assert_eq!(error.path(), Some("ports[2]"));
    assert_eq!(error.source_index(), Some(2));
}
