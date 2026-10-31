// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Integration tests for the custom serde deserializer.

use std::collections::HashMap;
use std::fmt;

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::ConfigSerdeExt;
use qubit_config::Property;
use qubit_config::UnknownFieldPolicy;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::CollectionConversionLimits;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_datatype::ConversionPolicy;
use qubit_datatype::DataConversionErrorKind;
use qubit_datatype::DataConverter;
use qubit_datatype::DataType;
use qubit_datatype::EmptyItemPolicy;
use qubit_datatype::StructuredConversionLimits;
use qubit_value::MultiValues;
use qubit_value::Value as QubitValue;
use qubit_value::ValueContainer;
use serde::Deserialize;
use serde::Deserializer;
use serde::de;
use serde::de::Error;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;

/// Scalar-list diagnostics retain positions before empty items are skipped.
#[test]
fn test_scalar_list_error_preserves_original_index() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    config.set("case.value", "1,,bad")?;
    let error = config
        .deserialize::<OneField<Vec<u8>>>("case")
        .expect_err("the third raw item is not an integer");
    assert_eq!(error.path(), Some("case.value[2]"));
    Ok(())
}

/// Conversion stops at the first invalid retained item, before scanning tails.
#[test]
fn test_scalar_list_conversion_precedes_tail_validation() -> ConfigResult<()> {
    let mut config = Config::new();
    let policy = ReadPolicy::builder()
        .conversion_policy(ConversionPolicy::env_friendly())
        .empty_item_policy(EmptyItemPolicy::Reject)
        .build();
    config.set_default_read_policy(policy);
    config.set("case.value", "bad,,2")?;
    let error = config
        .deserialize::<OneField<Vec<u8>>>("case")
        .expect_err("the first invalid integer must fail before the empty tail item");
    assert_eq!(error.path(), Some("case.value[0]"));
    Ok(())
}

/// A custom consumer may finish after one item without validating the tail.
#[test]
fn test_scalar_list_visitor_can_stop_before_tail_limit() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_policy(ConversionPolicy::env_friendly())
            .collection_limits(CollectionConversionLimits::builder().max_items(1).build())
            .empty_item_policy(EmptyItemPolicy::Reject)
            .build(),
    );
    config.set("case.value", "7,,invalid,9")?;
    let first = config.deserialize::<OneField<FirstScalarItem>>("case")?;
    assert_eq!(first.value, FirstScalarItem(7));
    Ok(())
}

/// Test-only consumer that intentionally ignores every element after the first.
#[derive(Debug, PartialEq)]
struct FirstScalarItem(u8);

impl<'de> Deserialize<'de> for FirstScalarItem {
    /// Requests sequence access and consumes its first retained element only.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FirstVisitor;
        impl<'de> Visitor<'de> for FirstVisitor {
            type Value = FirstScalarItem;

            /// Describes the nonempty scalar-list contract.
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a nonempty list")
            }

            /// Stops before requesting or validating the tail.
            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                sequence
                    .next_element::<u8>()?
                    .map(FirstScalarItem)
                    .ok_or_else(|| de::Error::custom("missing first item"))
            }
        }
        deserializer.deserialize_seq(FirstVisitor)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct SignedScalars {
    i8_value: i8,
    i16_value: i16,
    i32_value: i32,
    i64_value: i64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct UnsignedScalars {
    u8_value: u8,
    u16_value: u16,
    u32_value: u32,
    u64_value: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct WideIntegers {
    signed: i128,
    unsigned: u128,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FloatScalars {
    f32_value: f32,
    f64_value: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DerivedShapes {
    unit_value: (),
    unit_struct: UnitStruct,
    newtype: Newtype,
    tuple: (u8, u8),
    tuple_struct: Pair,
    mode: Mode,
    ch: char,
    ignored_source: i32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct UnitStruct;

#[derive(Debug, Deserialize, PartialEq)]
struct Newtype(u16);

#[derive(Debug, Deserialize, PartialEq)]
struct Pair(u8, u8);

#[derive(Debug, Deserialize, PartialEq)]
struct Endpoint {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct TypedCollections {
    servers: Vec<Endpoint>,
    named: HashMap<String, Endpoint>,
}

#[derive(Debug, PartialEq, Default)]
enum Presence {
    #[default]
    Missing,
    Null,
}

impl<'de> Deserialize<'de> for Presence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PresenceVisitor;

        impl<'de> Visitor<'de> for PresenceVisitor {
            type Value = Presence;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an optional value")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Presence::Null)
            }

            fn visit_some<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Err(de::Error::custom("expected null"))
            }
        }

        deserializer.deserialize_option(PresenceVisitor)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct OptionalValue {
    #[serde(default)]
    value: Presence,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Mode {
    Fast,
    Slow,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Tagged {
    Unit,
    Code(u16),
    Pair(u8, u8),
    Record { id: u8 },
}

#[derive(Debug, Deserialize, PartialEq)]
struct DirectEntryPoints {
    string_from_bool: String,
    string_from_number: String,
    string_from_unsigned: String,
    string_from_float: String,
    str_only: StrOnly,
    bytes_only: BytesOnly,
    byte_buf_only: ByteBufOnly,
    identifier: IdentifierOnly,
    map: HashMap<String, u8>,
    any_number: serde_json::Value,
    any_array: serde_json::Value,
    any_null: serde_json::Value,
}

#[derive(Debug, PartialEq)]
struct StrOnly(String);

impl<'de> Deserialize<'de> for StrOnly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrVisitor;

        impl Visitor<'_> for StrVisitor {
            type Value = StrOnly;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrOnly(value.to_string()))
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

#[derive(Debug, PartialEq)]
struct BytesOnly(Vec<u8>);

impl<'de> Deserialize<'de> for BytesOnly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl Visitor<'_> for BytesVisitor {
            type Value = BytesOnly;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bytes")
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BytesOnly(value))
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

#[derive(Debug, PartialEq)]
struct ByteBufOnly(Vec<u8>);

impl<'de> Deserialize<'de> for ByteBufOnly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ByteBufVisitor;

        impl Visitor<'_> for ByteBufVisitor {
            type Value = ByteBufOnly;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("byte buffer")
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ByteBufOnly(value))
            }
        }

        deserializer.deserialize_byte_buf(ByteBufVisitor)
    }
}

#[derive(Debug, PartialEq)]
struct IdentifierOnly(String);

impl<'de> Deserialize<'de> for IdentifierOnly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdentifierVisitor;

        impl Visitor<'_> for IdentifierVisitor {
            type Value = IdentifierOnly;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(IdentifierOnly(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(IdentifierOnly(value))
            }
        }

        deserializer.deserialize_identifier(IdentifierVisitor)
    }
}

#[derive(Debug, Deserialize)]
struct OneField<T> {
    #[allow(dead_code)]
    value: T,
}

#[derive(Debug, PartialEq)]
struct StringErrorProbe;

impl<'de> Deserialize<'de> for StringErrorProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer) {
            Ok(_) => Ok(Self),
            Err(error) => {
                let _ = error.to_string();
                let _ = std::error::Error::source(&error);
                Err(error)
            }
        }
    }
}

#[test]
fn deserialize_signed_scalars_from_strings_and_numbers() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("signed.i8_value", "8")?;
    config.set("signed.i16_value", 16i16)?;
    config.set("signed.i32_value", "32")?;
    config.set("signed.i64_value", 64i64)?;

    let actual: SignedScalars = config.deserialize("signed")?;

    assert_eq!(
        actual,
        SignedScalars {
            i8_value: 8,
            i16_value: 16,
            i32_value: 32,
            i64_value: 64,
        }
    );
    Ok(())
}

#[test]
fn deserialize_unsigned_scalars_from_strings_and_numbers() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("unsigned.u8_value", 8u8)?;
    config.set("unsigned.u16_value", "16")?;
    config.set("unsigned.u32_value", 32u32)?;
    config.set("unsigned.u64_value", "64")?;

    let actual: UnsignedScalars = config.deserialize("unsigned")?;

    assert_eq!(
        actual,
        UnsignedScalars {
            u8_value: 8,
            u16_value: 16,
            u32_value: 32,
            u64_value: 64,
        }
    );
    Ok(())
}

#[test]
fn test_deserialize_wide_integers() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("wide.signed", i128::MIN)?;
    config.set("wide.unsigned", u128::MAX)?;

    let actual: WideIntegers = config.deserialize("wide")?;

    assert_eq!(
        actual,
        WideIntegers {
            signed: i128::MIN,
            unsigned: u128::MAX,
        }
    );
    Ok(())
}

#[test]
fn test_deserialize_i128_rejects_invalid_redacted_value() -> ConfigResult<()> {
    const INVALID_VALUE: &str = "TOP_SECRET_INVALID_I128";
    let mut config = Config::new();
    config.set("wide.signed", INVALID_VALUE)?;
    config.set("wide.unsigned", 1u128)?;

    let error = config
        .deserialize::<WideIntegers>("wide")
        .expect_err("an invalid i128 must be rejected");

    assert!(!error.to_string().contains(INVALID_VALUE));
    assert!(!format!("{error:?}").contains(INVALID_VALUE));
    match error {
        ConfigError::ConversionError { key, .. } => {
            assert_eq!(key, "wide.signed");
        }
        other => panic!("expected a structured conversion error: {other}"),
    }
    Ok(())
}

#[test]
fn test_deserialize_u128_rejects_overflow() -> ConfigResult<()> {
    let overflow = format!("{}0", u128::MAX);
    let mut config = Config::new();
    config.set("wide.signed", 1i128)?;
    config.set("wide.unsigned", overflow.as_str())?;

    let error = config
        .deserialize::<WideIntegers>("wide")
        .expect_err("an overflowing u128 must be rejected");

    assert!(!error.to_string().contains(overflow.as_str()));
    assert!(!format!("{error:?}").contains(overflow.as_str()));
    match error {
        ConfigError::ConversionError { key, .. } => {
            assert_eq!(key, "wide.unsigned");
        }
        other => panic!("expected a structured conversion error: {other}"),
    }
    Ok(())
}

#[test]
fn deserialize_float_scalars_from_strings_and_numbers() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("floats.f32_value", "1.5")?;
    config.set("floats.f64_value", 2.5f64)?;

    let actual: FloatScalars = config.deserialize("floats")?;

    assert_eq!(
        actual,
        FloatScalars {
            f32_value: 1.5,
            f64_value: 2.5,
        }
    );
    Ok(())
}

#[test]
fn deserialize_numeric_scalars_cover_number_and_string_paths() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("i8_number.value", 8i8)?;
    config.set("i16_string.value", "16")?;
    config.set("i32_number.value", 32i32)?;
    config.set("i64_string.value", "64")?;
    config.set("u8_string.value", "8")?;
    config.set("u16_number.value", 16u16)?;
    config.set("u32_string.value", "32")?;
    config.set("u64_number.value", 64u64)?;
    config.set("f32_number.value", 1.5f32)?;
    config.set("f64_string.value", "2.5")?;

    assert_eq!(config.deserialize::<OneField<i8>>("i8_number")?.value, 8);
    assert_eq!(config.deserialize::<OneField<i16>>("i16_string")?.value, 16);
    assert_eq!(config.deserialize::<OneField<i32>>("i32_number")?.value, 32);
    assert_eq!(config.deserialize::<OneField<i64>>("i64_string")?.value, 64);
    assert_eq!(config.deserialize::<OneField<u8>>("u8_string")?.value, 8);
    assert_eq!(config.deserialize::<OneField<u16>>("u16_number")?.value, 16);
    assert_eq!(config.deserialize::<OneField<u32>>("u32_string")?.value, 32);
    assert_eq!(config.deserialize::<OneField<u64>>("u64_number")?.value, 64);
    assert_eq!(config.deserialize::<OneField<f32>>("f32_number")?.value, 1.5);
    assert_eq!(config.deserialize::<OneField<f64>>("f64_string")?.value, 2.5);
    Ok(())
}

#[test]
fn deserialize_derived_shapes() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_null("shapes.unit_value", DataType::String)?;
    config.set_null("shapes.unit_struct", DataType::String)?;
    config.set("shapes.newtype", "8080")?;
    config.set("shapes.tuple", vec![1u8, 2u8])?;
    config.set("shapes.tuple_struct", vec![3u8, 4u8])?;
    config.set("shapes.mode", "Fast")?;
    config.set("shapes.ch", "x")?;
    config.set("shapes.ignored_source", 42i32)?;
    config.set("shapes.extra", "ignored")?;

    let actual: DerivedShapes = config.deserialize_with(
        "shapes",
        ConfigDeserializeOptions {
            interpolate: false,
            unknown_fields: UnknownFieldPolicy::Ignore,
        },
    )?;

    assert_eq!(
        actual,
        DerivedShapes {
            unit_value: (),
            unit_struct: UnitStruct,
            newtype: Newtype(8080),
            tuple: (1, 2),
            tuple_struct: Pair(3, 4),
            mode: Mode::Fast,
            ch: 'x',
            ignored_source: 42,
        }
    );
    Ok(())
}

#[test]
fn deserialize_enum_string_uses_config_read_policy() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    config.set("mode.value", " Fast ")?;

    let actual = config.deserialize::<OneField<Mode>>("mode")?;

    assert_eq!(actual.value, Mode::Fast);
    Ok(())
}

/// Test character conversion errors never expose the configuration value.
#[test]
fn deserialize_char_redacts_secret_value() -> ConfigResult<()> {
    const SECRET: &str = "TOP_SECRET_CHAR_MARKER";
    let mut config = Config::new();
    config.set("secret.value", SECRET)?;

    let error = config
        .deserialize::<OneField<char>>("secret")
        .expect_err("a multi-character secret must not deserialize as char");

    assert!(!error.to_string().contains(SECRET));
    assert!(!format!("{error:?}").contains(SECRET));
    match error {
        ConfigError::ConversionError { key, source, .. }
            if key == "secret.value"
                && source.from_type() == Some(DataType::String)
                && source.to_type() == DataType::Char
                && source.kind() == DataConversionErrorKind::InvalidValue => {}
        other => panic!("expected structured char conversion error: {other}"),
    }
    Ok(())
}

/// Test unknown enum diagnostics never expose the configuration value.
#[test]
fn deserialize_unknown_enum_redacts_secret_value() -> ConfigResult<()> {
    const SECRET: &str = "TOP_SECRET_ENUM_MARKER";
    let mut config = Config::new();
    config.set("secret.value", SECRET)?;

    let error = config
        .deserialize::<OneField<Mode>>("secret")
        .expect_err("an unknown secret marker must not deserialize as Mode");

    assert!(!error.to_string().contains(SECRET));
    assert!(!format!("{error:?}").contains(SECRET));
    match error {
        ConfigError::DeserializeError {
            message, source: None, ..
        } => assert_eq!(message, "configuration value does not match the requested type",),
        other => panic!("expected redacted deserialization error: {other}"),
    }
    Ok(())
}

#[test]
fn deserialize_externally_tagged_enum_variants() -> ConfigResult<()> {
    let mut unit_config = Config::new();
    unit_config.set_null("case.value.Unit", DataType::String)?;
    let unit = unit_config.deserialize::<OneField<Tagged>>("case")?;

    let mut root_unit_config = Config::new();
    root_unit_config.set_null("Unit", DataType::String)?;
    let root_unit = root_unit_config.deserialize::<Tagged>("")?;

    let mut newtype_config = Config::new();
    newtype_config.set_default_read_policy(ReadPolicy::env_friendly());
    newtype_config.set("case.value.Code", " 200 ")?;
    let newtype = newtype_config.deserialize::<OneField<Tagged>>("case")?;

    let mut tuple_config = Config::new();
    tuple_config.set("case.value.Pair", vec![1u8, 2u8])?;
    let tuple = tuple_config.deserialize::<OneField<Tagged>>("case")?;

    let mut struct_config = Config::new();
    struct_config.set_default_read_policy(ReadPolicy::env_friendly());
    struct_config.set("case.value.Record.id", " 7 ")?;
    let record = struct_config.deserialize::<OneField<Tagged>>("case")?;

    assert_eq!(unit.value, Tagged::Unit);
    assert_eq!(root_unit, Tagged::Unit);
    assert_eq!(newtype.value, Tagged::Code(200));
    assert_eq!(tuple.value, Tagged::Pair(1, 2));
    assert_eq!(record.value, Tagged::Record { id: 7 });
    Ok(())
}

#[test]
fn deserialize_enum_reports_invalid_shapes() -> ConfigResult<()> {
    let mut empty_object = Config::new();
    empty_object.insert_property(
        "case.value",
        Property::new("case.value", QubitValue::Json(serde_json::json!({})))?,
    )?;
    assert!(empty_object.deserialize::<OneField<Tagged>>("case").is_err());

    let mut multiple_variants = Config::new();
    multiple_variants.insert_property(
        "case.value",
        Property::new(
            "case.value",
            QubitValue::Json(serde_json::json!({
                "Unit": null,
                "Code": 200
            })),
        )?,
    )?;
    assert!(multiple_variants.deserialize::<OneField<Tagged>>("case").is_err());

    let mut scalar = Config::new();
    scalar.set("case.value", 1u8)?;
    assert!(scalar.deserialize::<OneField<Tagged>>("case").is_err());

    let mut bad_unit_payload = Config::new();
    bad_unit_payload.insert_property(
        "case.value",
        Property::new(
            "case.value",
            MultiValues::Json(vec![serde_json::json!({
                "Unit": { "extra": true }
            })]),
        )?,
    )?;
    assert!(bad_unit_payload.deserialize::<OneField<Tagged>>("case").is_err());

    let mut missing_newtype = Config::new();
    missing_newtype.set("case.value", "Code")?;
    assert!(missing_newtype.deserialize::<OneField<Tagged>>("case").is_err());

    let mut missing_tuple = Config::new();
    missing_tuple.set("case.value", "Pair")?;
    assert!(missing_tuple.deserialize::<OneField<Tagged>>("case").is_err());

    let mut missing_struct = Config::new();
    missing_struct.set("case.value", "Record")?;
    assert!(missing_struct.deserialize::<OneField<Tagged>>("case").is_err());
    Ok(())
}

#[test]
fn deserialize_direct_scalar_entry_points() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("direct.string_from_bool", true)?;
    config.set("direct.string_from_number", 123i32)?;
    config.set("direct.string_from_unsigned", u64::MAX)?;
    config.set("direct.string_from_float", 1.25_f64)?;
    config.set("direct.str_only", "abc")?;
    config.set("direct.bytes_only", "abc")?;
    config.set("direct.byte_buf_only", "xyz")?;
    config.set("direct.identifier", "field")?;
    config.set("direct.map.a", 1u8)?;
    config.set("direct.any_number", u64::MAX)?;
    config.set("direct.any_array", vec![1u8, 2u8])?;
    config.set_null("direct.any_null", DataType::String)?;

    let actual: DirectEntryPoints = config.deserialize("direct")?;

    assert_eq!(actual.string_from_bool, "true");
    assert_eq!(actual.string_from_number, "123");
    assert_eq!(actual.string_from_unsigned, u64::MAX.to_string());
    assert_eq!(actual.string_from_float, "1.25");
    assert_eq!(actual.str_only.0, "abc");
    assert_eq!(actual.bytes_only.0, b"abc");
    assert_eq!(actual.byte_buf_only.0, b"xyz");
    assert_eq!(actual.identifier.0, "field");
    assert_eq!(actual.map.get("a"), Some(&1));
    assert_eq!(actual.any_number, serde_json::json!(u64::MAX));
    assert_eq!(actual.any_array, serde_json::json!([1, 2]));
    assert_eq!(actual.any_null, serde_json::Value::Null);
    Ok(())
}

#[test]
fn deserialize_scalar_lists_cover_admitted_serde_entry_points() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    config.set("strings.value", "alpha,beta")?;
    config.set("bools.value", "true,false")?;
    config.set("chars.value", "a,b")?;
    config.set("modes.value", "Fast,Slow")?;
    config.set("units.value", "not-unit")?;
    config.set("sequences.value", "1,2")?;
    config.set("maps.value", "a,b")?;

    assert_eq!(
        config.deserialize::<OneField<Vec<serde_json::Value>>>("strings")?.value,
        vec![serde_json::json!("alpha"), serde_json::json!("beta")]
    );
    assert_eq!(
        config.deserialize::<OneField<Vec<bool>>>("bools")?.value,
        vec![true, false]
    );
    assert_eq!(
        config.deserialize::<OneField<Vec<char>>>("chars")?.value,
        vec!['a', 'b']
    );
    assert_eq!(
        config.deserialize::<OneField<Vec<Mode>>>("modes")?.value,
        vec![Mode::Fast, Mode::Slow]
    );
    assert!(config.deserialize::<OneField<Vec<()>>>("units").is_err());
    assert!(config.deserialize::<OneField<Vec<Vec<u8>>>>("sequences").is_err());
    assert!(
        config
            .deserialize::<OneField<Vec<HashMap<String, u8>>>>("maps")
            .is_err()
    );

    let mut invalid = Config::new();
    invalid.set_default_read_policy(ReadPolicy::env_friendly());
    invalid.set("bools.value", "true,not-a-boolean")?;
    invalid.set("chars.value", "a,too-long")?;
    invalid.set("modes.value", "Fast,Turbo")?;
    assert!(invalid.deserialize::<OneField<Vec<bool>>>("bools").is_err());
    assert!(invalid.deserialize::<OneField<Vec<char>>>("chars").is_err());
    assert!(invalid.deserialize::<OneField<Vec<Mode>>>("modes").is_err());

    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_output_bytes(0).build())
        .build();
    let mut limited = Config::new();
    limited.set_default_read_policy(
        ReadPolicy::builder_from(&ReadPolicy::env_friendly())
            .conversion_limits(limits)
            .build(),
    );
    limited.set("value.value", "one,two")?;
    assert!(limited.deserialize::<OneField<Vec<String>>>("value").is_err());
    assert!(
        limited
            .deserialize::<OneField<Vec<serde_json::Value>>>("value")
            .is_err()
    );
    Ok(())
}

#[test]
fn deserialize_sequence_and_root_map_entry_points() -> ConfigResult<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct SequenceFields {
        vec_u8: Vec<u8>,
        tuple: (u8, u8),
        tuple_struct: Pair,
    }

    let mut sequence_config = Config::new();
    sequence_config.set_default_read_policy(ReadPolicy::env_friendly());
    sequence_config.set("seq.vec_u8", "1,2")?;
    sequence_config.set("seq.tuple", "3,4")?;
    sequence_config.set("seq.tuple_struct", "5,6")?;

    assert_eq!(
        sequence_config.deserialize::<SequenceFields>("seq")?,
        SequenceFields {
            vec_u8: vec![1, 2],
            tuple: (3, 4),
            tuple_struct: Pair(5, 6),
        }
    );

    let mut root_config = Config::new();
    root_config.set("a", 1u8)?;
    root_config.set("b", 2u8)?;
    assert_eq!(
        root_config.deserialize::<HashMap<String, u8>>("")?,
        HashMap::from([("a".to_string(), 1), ("b".to_string(), 2)])
    );
    assert!(root_config.deserialize::<HashMap<u32, u8>>("").is_err());

    let mut bad_map_config = Config::new();
    bad_map_config.set("value", true)?;
    assert!(bad_map_config.deserialize::<HashMap<String, u8>>("value").is_err());
    Ok(())
}

#[test]
fn map_key_boundary_preserves_numeric_names_as_strings() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("80", 8080_u16)?;
    let values: HashMap<String, u16> = config.deserialize("")?;
    assert_eq!(values.get("80"), Some(&8080));
    Ok(())
}

#[test]
fn map_key_boundary_rejects_numeric_target_at_root_path() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("80", 8080_u16)?;
    let error = config
        .deserialize::<HashMap<u32, u16>>("")
        .expect_err("source member names are offered to the key deserializer as strings");
    assert_eq!(error.path(), Some(""));
    Ok(())
}

#[test]
fn map_key_boundary_allows_explicit_parse_and_propagates_parse_errors() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("80", 8080_u16)?;
    let values: HashMap<String, u16> = config.deserialize("")?;
    let parsed: Result<HashMap<u32, u16>, _> = values
        .into_iter()
        .map(|(key, value)| key.parse::<u32>().map(|key| (key, value)))
        .collect();
    assert_eq!(parsed.expect("valid numeric key parses"), HashMap::from([(80, 8080)]));

    let mut invalid = Config::new();
    invalid.set("port", 8080_u16)?;
    let values: HashMap<String, u16> = invalid.deserialize("")?;
    let parse_error: Result<HashMap<u32, u16>, _> = values
        .into_iter()
        .map(|(key, value)| key.parse::<u32>().map(|key| (key, value)))
        .collect();
    assert!(parse_error.is_err());
    Ok(())
}

#[test]
fn deserialize_typed_vectors_and_maps_preserves_nested_shapes() -> ConfigResult<()> {
    let mut config = Config::new();
    config.insert_property(
        "collections.servers",
        Property::new(
            "collections.servers",
            serde_json::json!([
                {"host": "one", "port": 80},
                {"host": "two", "port": 443},
            ]),
        )?,
    )?;
    config.insert_property(
        "collections.named",
        Property::new(
            "collections.named",
            serde_json::json!({
                "primary": {"host": "one", "port": 80},
                "backup": {"host": "two", "port": 443},
            }),
        )?,
    )?;

    let actual: TypedCollections = config.deserialize_with(
        "collections",
        ConfigDeserializeOptions {
            interpolate: false,
            unknown_fields: UnknownFieldPolicy::Ignore,
        },
    )?;

    assert_eq!(
        actual,
        TypedCollections {
            servers: vec![
                Endpoint {
                    host: "one".to_owned(),
                    port: 80
                },
                Endpoint {
                    host: "two".to_owned(),
                    port: 443
                },
            ],
            named: HashMap::from([
                (
                    "primary".to_owned(),
                    Endpoint {
                        host: "one".to_owned(),
                        port: 80
                    }
                ),
                (
                    "backup".to_owned(),
                    Endpoint {
                        host: "two".to_owned(),
                        port: 443
                    }
                ),
            ]),
        }
    );
    Ok(())
}

#[test]
fn deserialize_distinguishes_null_from_missing_field() -> ConfigResult<()> {
    let mut null_config = Config::new();
    null_config.set_null("null.value", DataType::String)?;
    assert_eq!(
        null_config.deserialize_with::<OptionalValue>(
            "null",
            ConfigDeserializeOptions {
                interpolate: false,
                unknown_fields: UnknownFieldPolicy::Ignore
            }
        )?,
        OptionalValue { value: Presence::Null }
    );

    assert_eq!(
        Config::new().deserialize_with::<OptionalValue>(
            "missing",
            ConfigDeserializeOptions {
                interpolate: false,
                unknown_fields: UnknownFieldPolicy::Ignore
            }
        )?,
        OptionalValue {
            value: Presence::Missing
        }
    );
    Ok(())
}

#[test]
fn deserialize_unknown_scalar_shape_returns_type_error() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("value", vec![true])?;

    let error = config
        .deserialize::<OneField<u8>>("value")
        .expect_err("a sequence must not be converted to an integer");

    assert!(matches!(error, ConfigError::DeserializeError { .. }));
    Ok(())
}

#[test]
fn deserialize_one_field_success_for_error_only_types() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("string.value", "text")?;
    config.set("bool.value", true)?;
    config.set("char.value", "x")?;
    config.set_null("unit.value", DataType::String)?;
    config.set("vec_string.value", vec!["a", "b"])?;
    config.set("vec_u8.value", vec![1u8, 2u8])?;
    config.set("map.value.a", 1u8)?;
    config.set("str_only.value", "abc")?;
    config.set("bytes_only.value", "abc")?;
    config.set("byte_buf_only.value", "xyz")?;
    config.set("mode.value", "Fast")?;
    config.set("json.bool", true)?;

    assert_eq!(config.deserialize::<OneField<String>>("string")?.value, "text");
    assert!(config.deserialize::<OneField<bool>>("bool")?.value);
    assert_eq!(config.deserialize::<OneField<char>>("char")?.value, 'x');
    assert_eq!(config.deserialize::<OneField<()>>("unit")?.value, ());
    assert_eq!(
        config.deserialize::<OneField<Vec<String>>>("vec_string")?.value,
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(config.deserialize::<OneField<Vec<u8>>>("vec_u8")?.value, vec![1, 2]);
    assert_eq!(
        config
            .deserialize::<OneField<HashMap<String, u8>>>("map")?
            .value
            .get("a"),
        Some(&1)
    );
    assert_eq!(
        config.deserialize::<OneField<StrOnly>>("str_only")?.value,
        StrOnly("abc".to_string())
    );
    assert_eq!(
        config.deserialize::<OneField<BytesOnly>>("bytes_only")?.value,
        BytesOnly(b"abc".to_vec())
    );
    assert_eq!(
        config.deserialize::<OneField<ByteBufOnly>>("byte_buf_only")?.value,
        ByteBufOnly(b"xyz".to_vec())
    );
    assert_eq!(config.deserialize::<OneField<Mode>>("mode")?.value, Mode::Fast);
    assert_eq!(
        config.deserialize::<serde_json::Value>("json")?,
        serde_json::json!({ "bool": true })
    );
    Ok(())
}

#[test]
fn deserialize_struct_error_paths_for_success_only_types() {
    let config = Config::new();

    assert!(config.deserialize::<SignedScalars>("missing").is_err());
    assert!(config.deserialize::<UnsignedScalars>("missing").is_err());
    assert!(config.deserialize::<FloatScalars>("missing").is_err());
    assert!(config.deserialize::<DerivedShapes>("missing").is_err());
    assert!(config.deserialize::<DirectEntryPoints>("missing").is_err());
}

#[test]
fn deserialize_scalar_error_branches() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_null("null_string.value", DataType::String)?;
    config.set("array_string.value", vec![1u8, 2u8])?;
    config.set("object_string.value.a", 1u8)?;
    config.set("bool_number.value", 1u8)?;
    config.set("bool_string.value", "not-a-boolean")?;
    config.set("u8_overflow.value", 1000u16)?;
    config.set("seq_bool.value", true)?;
    config.set("map_bool.value", true)?;
    config.set("unit_bool.value", true)?;
    config.set("char_empty.value", "")?;
    config.set("char_long.value", "ab")?;
    config.set("char_bool.value", true)?;
    config.set_null("bool_null.value", DataType::String)?;
    config.set("unit_string.value", "unit")?;
    config.set("u8_array.value", vec![1u8, 2u8])?;
    config.set("bool_object.value.a", 1u8)?;
    config.set("i8_unsigned.value", u64::MAX)?;
    config.set("i8_overflow.value", 1000i64)?;
    config.set("i8_bad_string.value", "bad")?;
    config.set("i8_bool.value", true)?;
    config.set("i16_unsigned.value", u64::MAX)?;
    config.set("i16_overflow.value", 100000i64)?;
    config.set("i16_bool.value", true)?;
    config.set("i16_bad_string.value", "bad")?;
    config.set("i32_unsigned.value", u64::MAX)?;
    config.set("i32_overflow.value", i64::MAX)?;
    config.set("i32_bool.value", true)?;
    config.set("i32_bad_string.value", "bad")?;
    config.set("i64_unsigned.value", u64::MAX)?;
    config.set("i64_bool.value", true)?;
    config.set("i64_bad_string.value", "bad")?;
    config.set("u8_negative.value", -1i8)?;
    config.set("u8_bad_string.value", "bad")?;
    config.set("u16_negative.value", -1i8)?;
    config.set("u16_overflow.value", u64::MAX)?;
    config.set("u16_array.value", vec![1u8, 2u8])?;
    config.set("u16_bad_string.value", "bad")?;
    config.set("u32_negative.value", -1i8)?;
    config.set("u32_overflow.value", u64::MAX)?;
    config.set("u32_array.value", vec![1u8, 2u8])?;
    config.set("u32_bad_string.value", "bad")?;
    config.set("u64_negative.value", -1i8)?;
    config.set("u64_array.value", vec![1u8, 2u8])?;
    config.set("u64_bad_string.value", "bad")?;
    config.set("f32_bad_string.value", "bad")?;
    config.set("f32_bool.value", true)?;
    config.set("f64_bad_string.value", "bad")?;
    config.set("f64_bool.value", true)?;
    config.set("bad_enum.value", "Turbo")?;

    assert!(config.deserialize::<OneField<String>>("null_string").is_err());
    assert!(config.deserialize::<OneField<String>>("array_string").is_err());
    assert!(config.deserialize::<OneField<String>>("object_string").is_err());
    assert!(config.deserialize::<OneField<bool>>("bool_number")?.value);
    assert!(config.deserialize::<OneField<bool>>("bool_string").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_overflow").is_err());
    assert!(config.deserialize::<OneField<Vec<u8>>>("seq_bool").is_err());
    assert!(config.deserialize::<OneField<HashMap<String, u8>>>("map_bool").is_err());
    assert!(config.deserialize::<OneField<()>>("unit_bool").is_err());
    assert!(config.deserialize::<OneField<char>>("char_empty").is_err());
    assert!(config.deserialize::<OneField<char>>("char_long").is_err());
    assert!(config.deserialize::<OneField<char>>("char_bool").is_err());
    assert!(config.deserialize::<OneField<bool>>("bool_null").is_err());
    assert!(config.deserialize::<OneField<()>>("unit_string").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_array").is_err());
    assert!(config.deserialize::<OneField<bool>>("bool_object").is_err());
    assert!(config.deserialize::<OneField<i8>>("i8_unsigned").is_err());
    assert!(config.deserialize::<OneField<i8>>("i8_overflow").is_err());
    assert!(config.deserialize::<OneField<i8>>("i8_bad_string").is_err());
    assert_eq!(config.deserialize::<OneField<i8>>("i8_bool")?.value, 1);
    assert!(config.deserialize::<OneField<i16>>("i16_unsigned").is_err());
    assert!(config.deserialize::<OneField<i16>>("i16_overflow").is_err());
    assert_eq!(config.deserialize::<OneField<i16>>("i16_bool")?.value, 1);
    assert!(config.deserialize::<OneField<i16>>("i16_bad_string").is_err());
    assert!(config.deserialize::<OneField<i32>>("i32_unsigned").is_err());
    assert!(config.deserialize::<OneField<i32>>("i32_overflow").is_err());
    assert_eq!(config.deserialize::<OneField<i32>>("i32_bool")?.value, 1);
    assert!(config.deserialize::<OneField<i32>>("i32_bad_string").is_err());
    assert!(config.deserialize::<OneField<i64>>("i64_unsigned").is_err());
    assert_eq!(config.deserialize::<OneField<i64>>("i64_bool")?.value, 1);
    assert!(config.deserialize::<OneField<i64>>("i64_bad_string").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_negative").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_bad_string").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_negative").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_overflow").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_array").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_bad_string").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_negative").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_overflow").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_array").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_bad_string").is_err());
    assert!(config.deserialize::<OneField<u64>>("u64_negative").is_err());
    assert!(config.deserialize::<OneField<u64>>("u64_array").is_err());
    assert!(config.deserialize::<OneField<u64>>("u64_bad_string").is_err());
    assert!(config.deserialize::<OneField<f32>>("f32_bad_string").is_err());
    assert_eq!(config.deserialize::<OneField<f32>>("f32_bool")?.value, 1.0);
    assert!(config.deserialize::<OneField<f64>>("f64_bad_string").is_err());
    assert_eq!(config.deserialize::<OneField<f64>>("f64_bool")?.value, 1.0);
    assert!(config.deserialize::<OneField<Mode>>("bad_enum").is_err());
    Ok(())
}

#[test]
fn deserialize_read_option_error_branches() -> ConfigResult<()> {
    let mut blank_config = Config::new();
    blank_config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::Reject)
            .build(),
    );
    blank_config.set("blank_string.value", " ")?;
    blank_config.set("blank_bool.value", " ")?;
    blank_config.set("blank_str.value", " ")?;
    blank_config.set("blank_bytes.value", " ")?;
    blank_config.set("blank_byte_buf.value", " ")?;
    blank_config.set("blank_char.value", " ")?;
    blank_config.set("blank_seq.value", " ")?;
    blank_config.set("blank_any", " ")?;

    assert!(blank_config.deserialize::<OneField<String>>("blank_string").is_err());
    assert!(blank_config.deserialize::<OneField<bool>>("blank_bool").is_err());
    assert!(blank_config.deserialize::<OneField<StrOnly>>("blank_str").is_err());
    assert!(blank_config.deserialize::<OneField<BytesOnly>>("blank_bytes").is_err());
    assert!(
        blank_config
            .deserialize::<OneField<ByteBufOnly>>("blank_byte_buf")
            .is_err()
    );
    assert!(blank_config.deserialize::<OneField<char>>("blank_char").is_err());
    assert!(blank_config.deserialize::<OneField<Vec<String>>>("blank_seq").is_err());
    assert!(blank_config.deserialize::<serde_json::Value>("blank_any").is_err());

    let mut list_config = Config::new();
    list_config.set_default_read_policy(
        ReadPolicy::builder_from(&ReadPolicy::env_friendly())
            .empty_item_policy(EmptyItemPolicy::Reject)
            .build(),
    );
    list_config.set("bad_list.value", "a,,b")?;

    assert!(list_config.deserialize::<OneField<Vec<String>>>("bad_list").is_err());
    Ok(())
}

#[test]
fn deserialize_json_string_conversion_errors_use_config_read_policy() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::Reject)
            .build(),
    );
    config.insert_property(
        "string_value",
        Property::new("string_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "bool_value",
        Property::new("bool_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "list_value",
        Property::new("list_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "any_value",
        Property::new("any_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "char_value",
        Property::new("char_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "str_value",
        Property::new("str_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "bytes_value",
        Property::new("bytes_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    config.insert_property(
        "byte_buf_value",
        Property::new("byte_buf_value", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;

    assert!(config.deserialize::<String>("string_value").is_err());
    assert!(config.deserialize::<bool>("bool_value").is_err());
    assert!(config.deserialize::<Vec<String>>("list_value").is_err());
    assert!(config.deserialize::<serde_json::Value>("any_value").is_err());
    assert!(config.deserialize::<char>("char_value").is_err());
    assert!(config.deserialize::<StrOnly>("str_value").is_err());
    assert!(config.deserialize::<BytesOnly>("bytes_value").is_err());
    assert!(config.deserialize::<ByteBufOnly>("byte_buf_value").is_err());
    Ok(())
}

#[test]
fn deserialize_error_wrapper_formats_message_and_config_sources() -> ConfigResult<()> {
    let mut config_error = Config::new();
    config_error.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::Reject)
            .build(),
    );
    config_error.insert_property(
        "config_error",
        Property::new("config_error", MultiValues::Json(vec![serde_json::json!(" ")]))?,
    )?;
    assert!(config_error.deserialize::<StringErrorProbe>("config_error").is_err());

    let mut message_error = Config::new();
    message_error.set_null("message_error", DataType::String)?;
    assert!(message_error.deserialize::<StringErrorProbe>("message_error").is_err());
    Ok(())
}

#[test]
fn interpolation_admission_stops_before_later_bad_reference() -> ConfigResult<()> {
    let mut config = Config::new();
    let secret = "x".repeat(800);
    config.set("blob", secret.as_str())?;
    config.set("tree.a", "${blob}")?;
    config.set("tree.b", "${blob}")?;
    config.set("tree.z", "${missing}")?;
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_structured_payload_bytes(1601)
                .build(),
        )
        .build();
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());

    let error = config
        .deserialize_with::<serde_json::Value>(
            "tree",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect_err("expanded value should exceed payload budget before tree.z resolves");
    assert_eq!(error.path(), Some("tree.b"));
    assert!(!matches!(error, ConfigError::SubstitutionError { .. }));
    assert!(!error.to_string().contains(&secret));
    Ok(())
}

#[test]
fn interpolation_admission_charges_unchanged_text_after_a_changed_value() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("blob", "x".repeat(800))?;
    config.set("tree.a", "${blob}")?;
    config.set("tree.b", "y".repeat(800))?;
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_structured_payload_bytes(1599)
                .build(),
        )
        .build();
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());

    let error = config
        .deserialize_with::<serde_json::Value>(
            "tree",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect_err("unchanged text is admitted after expanded text consumes the budget");
    assert!(!matches!(error, ConfigError::SubstitutionError { .. }));
    assert_eq!(error.path(), Some("tree.b"));
    Ok(())
}

#[test]
fn interpolation_admission_visits_non_string_scalar_before_expansion() -> ConfigResult<()> {
    let mut scalar = Config::new();
    scalar.set("tree.flag", 12_345_678_901_234_567_890_u64)?;
    scalar.set("blob", "x".repeat(800))?;
    scalar.set("tree.value", "${blob}")?;
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_structured_payload_bytes(800)
                .build(),
        )
        .build();
    scalar.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
    let error = scalar
        .deserialize_with::<serde_json::Value>(
            "tree",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect_err("the expansion plus non-string scalar exceeds the payload budget");
    assert_eq!(error.path(), Some("tree.value"));
    Ok(())
}

#[test]
fn interpolation_admission_visits_container_before_expansion() -> ConfigResult<()> {
    let mut container = Config::new();
    container.insert_property(
        "tree.flag",
        Property::new("tree.flag", QubitValue::Json(serde_json::json!({"nested": [1]})))?,
    )?;
    let error = interpolation_admission_with_prior_value(&mut container, "tree")?;
    assert_eq!(error.path(), Some("tree.value"));
    Ok(())
}

#[test]
fn interpolation_admission_charges_long_key_before_expansion() -> ConfigResult<()> {
    let mut key = Config::new();
    key.set(format!("tree.{}", "k".repeat(12)), true)?;
    let error = interpolation_admission_with_prior_value(&mut key, "tree")?;
    assert_eq!(error.path(), Some("tree.value"));
    Ok(())
}

fn interpolation_admission_with_prior_value(config: &mut Config, prefix: &str) -> ConfigResult<ConfigError> {
    config.set("blob", "x".repeat(800))?;
    config.set("tree.value", "${blob}")?;
    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_input_bytes(807).build())
        .build();
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
    Ok(config
        .deserialize_with::<serde_json::Value>(
            prefix,
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect_err("the expansion should exceed the remaining input budget"))
}

#[test]
fn interpolation_admission_precedes_later_bad_reference_for_both_unknown_policies() -> ConfigResult<()> {
    for unknown_fields in [UnknownFieldPolicy::Reject, UnknownFieldPolicy::Ignore] {
        let mut config = Config::new();
        let secret = "x".repeat(800);
        config.set("blob", secret.as_str())?;
        config.set("tree.a", "${blob}")?;
        config.set("tree.b", "${blob}")?;
        config.set("tree.z", "${missing}")?;
        let limits = ConversionLimits::builder()
            .operation_limits(
                ConversionOperationLimits::builder()
                    .max_structured_payload_bytes(1599)
                    .build(),
            )
            .build();
        config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());

        let error = config
            .deserialize_with::<serde_json::Value>(
                "tree",
                ConfigDeserializeOptions {
                    interpolate: true,
                    unknown_fields,
                },
            )
            .expect_err("admission should stop before resolving the later bad reference");
        assert!(!matches!(error, ConfigError::SubstitutionError { .. }));
        assert_eq!(error.path(), Some("tree.b"));
    }
    Ok(())
}

#[test]
fn interpolation_admission_payload_limit_accepts_1602_bytes() -> ConfigResult<()> {
    interpolation_admission_boundary(true, 1602)
}

#[test]
fn interpolation_admission_payload_limit_rejects_1601_bytes() -> ConfigResult<()> {
    interpolation_admission_boundary(true, 1601)
}

#[test]
fn interpolation_admission_input_limit_accepts_1602_bytes() -> ConfigResult<()> {
    interpolation_admission_boundary(false, 1602)
}

#[test]
fn interpolation_admission_input_limit_rejects_1601_bytes() -> ConfigResult<()> {
    interpolation_admission_boundary(false, 1601)
}

#[test]
fn interpolation_admission_original_input_precedes_bad_reference() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("tree.value", "${missing}")?;
    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_input_bytes(0).build())
        .build();
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());

    let error = config
        .deserialize_with::<serde_json::Value>(
            "tree",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect_err("original input admission should fail before placeholder resolution");
    assert!(!matches!(error, ConfigError::SubstitutionError { .. }));
    Ok(())
}

fn interpolation_admission_boundary(payload: bool, limit: u64) -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("blob", "x".repeat(800))?;
    config.set("tree.a", "${blob}")?;
    config.set("tree.b", "${blob}")?;
    let operation = if payload {
        ConversionOperationLimits::builder()
            .max_structured_payload_bytes(limit)
            .build()
    } else {
        ConversionOperationLimits::builder().max_input_bytes(limit).build()
    };
    let limits = ConversionLimits::builder().operation_limits(operation).build();
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());

    let result = config.deserialize_with::<serde_json::Value>(
        "tree",
        ConfigDeserializeOptions {
            interpolate: true,
            unknown_fields: UnknownFieldPolicy::Ignore,
        },
    );
    if limit == 1602 {
        assert!(result.is_ok(), "the exact expanded boundary should be accepted");
    } else {
        let error = result.expect_err("the expanded value should exceed the configured limit");
        assert_eq!(error.path(), Some("tree.b"));
    }
    Ok(())
}

fn with_limits(config: &mut Config, limits: ConversionLimits) {
    config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
}

fn deserialize_with_options<T: for<'de> Deserialize<'de>>(
    config: &Config,
    prefix: &str,
    interpolate: bool,
    reject_unknown: bool,
) -> ConfigResult<T> {
    ConfigSerdeExt::deserialize_with(
        config,
        prefix,
        ConfigDeserializeOptions {
            interpolate,
            unknown_fields: if reject_unknown {
                UnknownFieldPolicy::Reject
            } else {
                UnknownFieldPolicy::Ignore
            },
        },
    )
}

#[test]
fn test_wide_numbers_remain_exact_and_any_uses_natural_text() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wide {
        low: i128,
        high: u128,
    }
    let mut config = Config::new();
    config.set("wide.low", i128::MIN).unwrap();
    config.set("wide.high", u128::MAX).unwrap();
    let actual: Wide = deserialize_with_options(&config, "wide", false, true).unwrap();
    assert_eq!(
        actual,
        Wide {
            low: i128::MIN,
            high: u128::MAX
        }
    );
    let actual: serde_json::Value = deserialize_with_options(&config, "wide", false, true).unwrap();
    assert_eq!(
        actual,
        serde_json::json!({"low": i128::MIN.to_string(), "high": u128::MAX.to_string()})
    );
}
#[test]
fn test_typed_numeric_errors_retain_the_original_source_type() {
    let mut config = Config::new();
    config.set("number", 1.5_f64).unwrap();
    let expected = DataConverter::from(1.5_f64)
        .to_with::<u8>(
            config.read_policy().conversion_policy(),
            config.read_policy().conversion_limits(),
        )
        .unwrap_err();
    let error = deserialize_with_options::<u8>(&config, "number", false, true).unwrap_err();
    assert!(matches!(error, ConfigError::ConversionError { key, source, .. } if key == "number" && source == expected));
}
#[test]
fn test_float32_any_keeps_natural_precision() {
    let mut config = Config::new();
    config.set("number", 0.1_f32).unwrap();
    let typed: f32 = deserialize_with_options(&config, "number", false, true).unwrap();
    assert_eq!(typed.to_bits(), 0.1_f32.to_bits());
    let any: serde_json::Value = deserialize_with_options(&config, "number", false, true).unwrap();
    assert_eq!(any, serde_json::json!(0.1));
}
#[test]
fn test_options_keep_unset_null_and_empty_collections_distinct() {
    let mut config = Config::new();
    config.set_null("unset", DataType::Int32).unwrap();
    config.set("null", serde_json::Value::Null).unwrap();
    config.set("empty", Vec::<i32>::new()).unwrap();
    assert_eq!(
        deserialize_with_options::<Option<i32>>(&config, "unset", false, true).unwrap(),
        None
    );
    assert_eq!(
        deserialize_with_options::<Option<i32>>(&config, "null", false, true).unwrap(),
        None
    );
    assert_eq!(
        deserialize_with_options::<Option<Vec<i32>>>(&config, "empty", false, true).unwrap(),
        Some(vec![])
    );
    let error = deserialize_with_options::<i32>(&config, "unset", false, true).unwrap_err();
    assert!(error.value_missing().unwrap().is_unset());
    assert_eq!(error.value_missing().unwrap().target_type(), Some(DataType::Int32));
}
#[test]
fn test_scalar_split_has_one_admission_and_collection_strings_never_split_again() {
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
    let values: Vec<u16> = deserialize_with_options(&config, "ports", false, true).unwrap();
    assert_eq!(values, [8080, 8081]);
    config.set("ports", vec!["1,2"]).unwrap();
    assert!(deserialize_with_options::<Vec<Vec<u16>>>(&config, "ports", false, true).is_err());
}
#[test]
fn test_all_numeric_and_any_leaves_share_one_session() {
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
        deserialize_with_options::<Pair>(&config, "", false, true)
            .unwrap_err()
            .to_string()
            .contains("Items")
    );
    assert!(
        deserialize_with_options::<serde_json::Value>(&config, "", false, true)
            .unwrap_err()
            .to_string()
            .contains("Items")
    );
}
#[test]
fn test_nested_newtypes_do_not_repeat_leaf_admission() {
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
        deserialize_with_options::<Outer>(&config, "text", false, true).unwrap(),
        Outer(Inner("ab".into()))
    );
}
#[test]
fn test_external_enum_variants_and_nested_paths_are_preserved() {
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
            deserialize_with_options::<Mode>(&config, "mode", false, true).unwrap(),
            expected
        );
    }
    config
        .set("mode", serde_json::json!({"Named": {"port": "secret-invalid"}}))
        .unwrap();
    let error = deserialize_with_options::<Mode>(&config, "mode", false, true).unwrap_err();
    assert_eq!(error.path(), Some("mode.Named.port"));
    assert!(!error.to_string().contains("secret-invalid"));
}
#[test]
fn test_ignored_fields_are_reported_or_admitted_without_consumption() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Known {
        port: u16,
    }
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.ignored", "x".repeat(65536)).unwrap();
    assert_eq!(
        deserialize_with_options::<Known>(&config, "server", false, false).unwrap(),
        Known { port: 8080 }
    );
    let error = deserialize_with_options::<Known>(&config, "server", false, true).unwrap_err();
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
    assert!(deserialize_with_options::<Known>(&config, "server", false, false).is_err());
}
#[test]
fn test_collection_and_split_failures_keep_original_indices() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    for value in [
        ValueContainer::from("1, secret-invalid, 3"),
        ValueContainer::from(vec!["1", "secret-invalid", "3"]),
    ] {
        config.set("values", value).unwrap();
        let error = deserialize_with_options::<Vec<u16>>(&config, "values", false, true).unwrap_err();
        assert_eq!(error.path(), Some("values[1]"));
        assert_eq!(error.source_index(), Some(1));
        assert!(!error.to_string().contains("secret-invalid"));
    }
}
#[test]
fn test_tuple_lengths_and_collection_missing_are_not_silently_ignored() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    for value in [ValueContainer::from(vec![1_i32, 2, 3]), ValueContainer::from("1,2,3")] {
        config.set("values", value).unwrap();
        assert!(deserialize_with_options::<(i32, i32)>(&config, "values", false, true).is_err());
    }
    config.set_default_read_policy(
        ReadPolicy::builder()
            .blank_string_policy(BlankStringPolicy::TreatAsMissing)
            .build(),
    );
    config.set("values", vec!["1", " "]).unwrap();
    let error = deserialize_with_options::<Vec<Option<i32>>>(&config, "values", false, true).unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert!(!error.value_missing().unwrap().is_defaultable_for_conversion());
}

#[test]
fn tuple_diagnostics_report_known_native_and_scalar_lengths() -> ConfigResult<()> {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct RequiredPair {
        pair: (i32, i32),
    }

    for (values, expected_message) in [
        (vec![], "length 0"),
        (vec![1_i32], "length 1"),
        (vec![1_i32, 2], ""),
        (vec![1_i32, 2, 3], "length 3"),
    ] {
        let mut config = Config::new();
        config.set("app.pair", values)?;
        if expected_message.is_empty() {
            assert_eq!(config.deserialize::<(i32, i32)>("app.pair")?, (1, 2));
        } else {
            let error = config
                .deserialize::<(i32, i32)>("app.pair")
                .expect_err("native collection length must match the tuple");
            assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Deserialize);
            assert_eq!(error.path(), Some("app.pair"));
            assert!(error.to_string().contains(expected_message));
            assert!(error.to_string().contains("expected 2"));
        }
    }

    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    config.set("app.pair", "1,2,3")?;
    let error = config
        .deserialize::<RequiredPair>("app")
        .expect_err("a split scalar with an extra tuple item must fail");
    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Deserialize);
    assert_eq!(error.path(), Some("app.pair"));
    assert!(error.to_string().contains("more elements"));
    assert!(error.to_string().contains("expected length 2"));
    Ok(())
}

#[test]
fn map_and_sequence_shape_errors_describe_expected_shape() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("value", true)?;
    let map_error = config
        .deserialize::<HashMap<String, u8>>("value")
        .expect_err("a scalar cannot be deserialized as a map");
    assert_eq!(map_error.path(), Some("value"));
    assert!(map_error.to_string().contains("shape: a map"));

    config.set("value", true)?;
    let sequence_error = config
        .deserialize::<Vec<u8>>("value")
        .expect_err("a boolean cannot be deserialized as a sequence");
    assert_eq!(sequence_error.path(), Some("value"));
    assert!(sequence_error.to_string().contains("shape: a sequence"));
    Ok(())
}
#[test]
fn test_interpolation_uses_overlays_for_nested_maps_sequences_and_missing_options() {
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
        deserialize_with_options::<Settings>(&config, "settings", true, true).unwrap(),
        Settings {
            ports: vec![8080],
            title: None
        }
    );
    config
        .set("json", serde_json::json!({"ports": ["${port}"], "title": "hello"}))
        .unwrap();
    assert_eq!(
        deserialize_with_options::<Settings>(&config, "json", true, true).unwrap(),
        Settings {
            ports: vec![8080],
            title: Some("hello".into())
        }
    );
}
#[test]
fn test_standard_duration_requires_its_actual_serde_structure() {
    let mut config = Config::new();
    config.set("duration", std::time::Duration::from_secs(2)).unwrap();
    assert!(deserialize_with_options::<std::time::Duration>(&config, "duration", false, true).is_err());
    config
        .set("duration", serde_json::json!({"secs": 2, "nanos": 3}))
        .unwrap();
    assert_eq!(
        deserialize_with_options::<std::time::Duration>(&config, "duration", false, true).unwrap(),
        std::time::Duration::new(2, 3)
    );
}
#[cfg(feature = "chrono")]
#[test]
fn test_chrono_targets_receive_their_requested_string_representation() {
    let mut config = Config::new();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    config.set("date", date).unwrap();
    assert_eq!(
        deserialize_with_options::<chrono::NaiveDate>(&config, "date", false, true).unwrap(),
        date
    );
}
#[cfg(feature = "bigdecimal")]
#[test]
fn test_bigdecimal_target_accepts_the_natural_rich_text() {
    let mut config = Config::new();
    let value: bigdecimal::BigDecimal = "1234567890.123456789".parse().unwrap();
    config.set("decimal", value.clone()).unwrap();
    assert_eq!(
        deserialize_with_options::<bigdecimal::BigDecimal>(&config, "decimal", false, true).unwrap(),
        value
    );
}
#[test]
fn test_byte_visitors_receive_utf8_from_string_compatible_scalars() {
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
        deserialize_with_options::<Bytes>(&config, "text", false, true).unwrap(),
        Bytes("中文".as_bytes().to_vec())
    );
}
#[test]
fn test_map_value_requested_before_key_returns_an_error() {
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
        let error = deserialize_with_options::<WrongOrder>(&config, "tree", false, reject_unknown).unwrap_err();
        assert_eq!(error.path(), Some("tree"));
    }
}
#[test]
fn test_invalid_map_keys_and_enum_payloads_preserve_paths() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Mode {
        Off,
        Count(u16),
        Pair(u16, String),
    }
    let mut config = Config::new();
    config.set("tree.key", "value").unwrap();
    let error =
        deserialize_with_options::<std::collections::BTreeMap<u32, String>>(&config, "tree", false, false).unwrap_err();
    assert_eq!(error.path(), Some("tree"));
    for (value, path) in [
        (serde_json::json!({"Unknown": null}), "mode.Unknown"),
        (serde_json::json!({"Count": "secret-invalid"}), "mode.Count"),
        (serde_json::json!({"Pair": ["secret-invalid", "text"]}), "mode.Pair[0]"),
    ] {
        config.set("mode", value).unwrap();
        let error = deserialize_with_options::<Mode>(&config, "mode", false, false).unwrap_err();
        assert_eq!(error.path(), Some(path));
        assert!(!error.to_string().contains("secret-invalid"));
    }
    config.set("mode", serde_json::json!({"Off": null})).unwrap();
    assert_eq!(
        deserialize_with_options::<Mode>(&config, "mode", false, false).unwrap(),
        Mode::Off
    );
}
#[test]
fn test_scalar_sequence_source_and_unconsumed_tail_limits_preserve_errors() {
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
    let error = deserialize_with_options::<Vec<u16>>(&config, "ports", false, false).unwrap_err();
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
    let error = deserialize_with_options::<(u16, u16)>(&config, "ports", false, false).unwrap_err();
    assert_eq!(error.path(), Some("ports[2]"));
    assert_eq!(error.source_index(), Some(2));
}
#[test]
fn test_index_and_nested_payload_respect_depth_and_node_limits() {
    let mut config = Config::new();
    config.set("tree.a.b", 1_i32).unwrap();
    config.set("json", serde_json::json!({"a": {"b": 1}})).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_depth(2).build())
            .build(),
    );
    assert!(config.deserialize::<serde_json::Value>("tree").is_err());
    assert!(config.deserialize::<serde_json::Value>("json").is_err());
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_structured_nodes(2).build())
            .build(),
    );
    assert!(config.deserialize::<serde_json::Value>("json").is_err());
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_structured_nodes(3).build())
            .build(),
    );
    assert!(config.deserialize::<serde_json::Value>("json").is_ok());
}
#[test]
fn test_nonfinite_numbers_are_rejected_before_target_selection() {
    let mut config = Config::new();
    config.set("ignored", vec![1.0_f64, f64::NAN]).unwrap();
    let error = config.deserialize::<serde_json::Value>("").unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert_eq!(error.path(), Some("ignored[1]"));
}
#[test]
fn test_deep_json_is_rejected_by_iterative_admission() {
    let mut value = serde_json::json!(true);
    for _ in 0..512 {
        value = serde_json::Value::Array(vec![value]);
    }
    let mut config = Config::new();
    config.set("deep", value).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_depth(8).build())
            .build(),
    );
    let error = config.deserialize::<serde_json::Value>("deep").unwrap_err();
    assert!(error.to_string().contains("StructuredDepth"));
}
#[test]
fn test_collection_budget_failure_retains_source_index_without_payload() {
    let mut config = Config::new();
    config.set("items", vec!["ok", "secret payload"]).unwrap();
    with_limits(
        &mut config,
        ConversionLimits::builder()
            .structured_limits(StructuredConversionLimits::builder().max_text_bytes(2).build())
            .build(),
    );
    let error = config.deserialize::<serde_json::Value>("items").unwrap_err();
    assert_eq!(error.source_index(), Some(1));
    assert_eq!(error.path(), Some("items[1]"));
    assert!(!error.to_string().contains("secret payload"));
}
