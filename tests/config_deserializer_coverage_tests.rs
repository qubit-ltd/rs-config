/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Integration coverage for the custom serde deserializer.

use std::collections::HashMap;
use std::fmt;

use qubit_config::{
    Config, ConfigResult, Property,
    options::{BlankStringPolicy, ConfigReadOptions, EmptyItemPolicy},
};
use qubit_datatype::DataType;
use qubit_value::MultiValues;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

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
enum Mode {
    Fast,
    Slow,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DirectEntryPoints {
    string_from_bool: String,
    string_from_number: String,
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
    assert_eq!(
        config.deserialize::<OneField<f32>>("f32_number")?.value,
        1.5
    );
    assert_eq!(
        config.deserialize::<OneField<f64>>("f64_string")?.value,
        2.5
    );
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

    let actual: DerivedShapes = config.deserialize("shapes")?;

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
fn deserialize_enum_string_uses_config_read_options() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_read_options(ConfigReadOptions::env_friendly());
    config.set("mode.value", " Fast ")?;

    let actual = config.deserialize::<OneField<Mode>>("mode")?;

    assert_eq!(actual.value, Mode::Fast);
    Ok(())
}

#[test]
fn deserialize_direct_scalar_entry_points() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("direct.string_from_bool", true)?;
    config.set("direct.string_from_number", 123i32)?;
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
fn deserialize_sequence_and_root_map_entry_points() -> ConfigResult<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct SequenceFields {
        vec_u8: Vec<u8>,
        tuple: (u8, u8),
        tuple_struct: Pair,
    }

    let mut sequence_config = Config::new();
    sequence_config.set_read_options(ConfigReadOptions::env_friendly());
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

    let mut bad_map_config = Config::new();
    bad_map_config.set("value", true)?;
    assert!(
        bad_map_config
            .deserialize::<HashMap<String, u8>>("value")
            .is_err()
    );
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

    assert_eq!(
        config.deserialize::<OneField<String>>("string")?.value,
        "text"
    );
    assert!(config.deserialize::<OneField<bool>>("bool")?.value);
    assert_eq!(config.deserialize::<OneField<char>>("char")?.value, 'x');
    assert_eq!(config.deserialize::<OneField<()>>("unit")?.value, ());
    assert_eq!(
        config
            .deserialize::<OneField<Vec<String>>>("vec_string")?
            .value,
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(
        config.deserialize::<OneField<Vec<u8>>>("vec_u8")?.value,
        vec![1, 2]
    );
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
        config
            .deserialize::<OneField<BytesOnly>>("bytes_only")?
            .value,
        BytesOnly(b"abc".to_vec())
    );
    assert_eq!(
        config
            .deserialize::<OneField<ByteBufOnly>>("byte_buf_only")?
            .value,
        ByteBufOnly(b"xyz".to_vec())
    );
    assert_eq!(
        config.deserialize::<OneField<Mode>>("mode")?.value,
        Mode::Fast
    );
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

    assert!(
        config
            .deserialize::<OneField<String>>("null_string")
            .is_err()
    );
    assert!(
        config
            .deserialize::<OneField<String>>("array_string")
            .is_err()
    );
    assert!(
        config
            .deserialize::<OneField<String>>("object_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<bool>>("bool_number").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_overflow").is_err());
    assert!(config.deserialize::<OneField<Vec<u8>>>("seq_bool").is_err());
    assert!(
        config
            .deserialize::<OneField<HashMap<String, u8>>>("map_bool")
            .is_err()
    );
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
    assert!(config.deserialize::<OneField<i8>>("i8_bool").is_err());
    assert!(config.deserialize::<OneField<i16>>("i16_unsigned").is_err());
    assert!(config.deserialize::<OneField<i16>>("i16_overflow").is_err());
    assert!(config.deserialize::<OneField<i16>>("i16_bool").is_err());
    assert!(
        config
            .deserialize::<OneField<i16>>("i16_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<i32>>("i32_unsigned").is_err());
    assert!(config.deserialize::<OneField<i32>>("i32_overflow").is_err());
    assert!(config.deserialize::<OneField<i32>>("i32_bool").is_err());
    assert!(
        config
            .deserialize::<OneField<i32>>("i32_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<i64>>("i64_unsigned").is_err());
    assert!(config.deserialize::<OneField<i64>>("i64_bool").is_err());
    assert!(
        config
            .deserialize::<OneField<i64>>("i64_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<u8>>("u8_negative").is_err());
    assert!(config.deserialize::<OneField<u8>>("u8_bad_string").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_negative").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_overflow").is_err());
    assert!(config.deserialize::<OneField<u16>>("u16_array").is_err());
    assert!(
        config
            .deserialize::<OneField<u16>>("u16_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<u32>>("u32_negative").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_overflow").is_err());
    assert!(config.deserialize::<OneField<u32>>("u32_array").is_err());
    assert!(
        config
            .deserialize::<OneField<u32>>("u32_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<u64>>("u64_negative").is_err());
    assert!(config.deserialize::<OneField<u64>>("u64_array").is_err());
    assert!(
        config
            .deserialize::<OneField<u64>>("u64_bad_string")
            .is_err()
    );
    assert!(
        config
            .deserialize::<OneField<f32>>("f32_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<f32>>("f32_bool").is_err());
    assert!(
        config
            .deserialize::<OneField<f64>>("f64_bad_string")
            .is_err()
    );
    assert!(config.deserialize::<OneField<f64>>("f64_bool").is_err());
    assert!(config.deserialize::<OneField<Mode>>("bad_enum").is_err());
    Ok(())
}

#[test]
fn deserialize_read_option_error_branches() -> ConfigResult<()> {
    let mut blank_config = Config::new();
    blank_config.set_read_options(
        ConfigReadOptions::default().with_blank_string_policy(BlankStringPolicy::Reject),
    );
    blank_config.set("blank_string.value", " ")?;
    blank_config.set("blank_bool.value", " ")?;
    blank_config.set("blank_str.value", " ")?;
    blank_config.set("blank_bytes.value", " ")?;
    blank_config.set("blank_byte_buf.value", " ")?;
    blank_config.set("blank_char.value", " ")?;
    blank_config.set("blank_seq.value", " ")?;
    blank_config.set("blank_any", " ")?;

    assert!(
        blank_config
            .deserialize::<OneField<String>>("blank_string")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<bool>>("blank_bool")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<StrOnly>>("blank_str")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<BytesOnly>>("blank_bytes")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<ByteBufOnly>>("blank_byte_buf")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<char>>("blank_char")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<OneField<Vec<String>>>("blank_seq")
            .is_err()
    );
    assert!(
        blank_config
            .deserialize::<serde_json::Value>("blank_any")
            .is_err()
    );

    let mut list_config = Config::new();
    list_config.set_read_options(
        ConfigReadOptions::env_friendly().with_empty_item_policy(EmptyItemPolicy::Reject),
    );
    list_config.set("bad_list.value", "a,,b")?;

    assert!(
        list_config
            .deserialize::<OneField<Vec<String>>>("bad_list")
            .is_err()
    );
    Ok(())
}

#[test]
fn deserialize_json_string_conversion_errors_use_config_read_options() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set_read_options(
        ConfigReadOptions::default().with_blank_string_policy(BlankStringPolicy::Reject),
    );
    config.insert_property(
        "string_value",
        Property::with_value(
            "string_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    config.insert_property(
        "bool_value",
        Property::with_value(
            "bool_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    config.insert_property(
        "list_value",
        Property::with_value(
            "list_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    config.insert_property(
        "any_value",
        Property::with_value("any_value", MultiValues::Json(vec![serde_json::json!(" ")])),
    )?;
    config.insert_property(
        "char_value",
        Property::with_value(
            "char_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    config.insert_property(
        "str_value",
        Property::with_value("str_value", MultiValues::Json(vec![serde_json::json!(" ")])),
    )?;
    config.insert_property(
        "bytes_value",
        Property::with_value(
            "bytes_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    config.insert_property(
        "byte_buf_value",
        Property::with_value(
            "byte_buf_value",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;

    assert!(config.deserialize::<String>("string_value").is_err());
    assert!(config.deserialize::<bool>("bool_value").is_err());
    assert!(config.deserialize::<Vec<String>>("list_value").is_err());
    assert!(
        config
            .deserialize::<serde_json::Value>("any_value")
            .is_err()
    );
    assert!(config.deserialize::<char>("char_value").is_err());
    assert!(config.deserialize::<StrOnly>("str_value").is_err());
    assert!(config.deserialize::<BytesOnly>("bytes_value").is_err());
    assert!(config.deserialize::<ByteBufOnly>("byte_buf_value").is_err());
    Ok(())
}

#[test]
fn deserialize_error_wrapper_formats_message_and_config_sources() -> ConfigResult<()> {
    let mut config_error = Config::new();
    config_error.set_read_options(
        ConfigReadOptions::default().with_blank_string_policy(BlankStringPolicy::Reject),
    );
    config_error.insert_property(
        "config_error",
        Property::with_value(
            "config_error",
            MultiValues::Json(vec![serde_json::json!(" ")]),
        ),
    )?;
    assert!(
        config_error
            .deserialize::<StringErrorProbe>("config_error")
            .is_err()
    );

    let mut message_error = Config::new();
    message_error.set_null("message_error", DataType::String)?;
    assert!(
        message_error
            .deserialize::<StringErrorProbe>("message_error")
            .is_err()
    );
    Ok(())
}
