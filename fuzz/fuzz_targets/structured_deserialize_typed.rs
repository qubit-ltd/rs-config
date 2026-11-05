// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes typed structured deserialization through representative Serde
//! visitor shapes.

use std::hint::black_box;
use std::time::Duration;

use libfuzzer_sys::fuzz_target;
use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::UnknownFieldPolicy;
use qubit_config::conversion::ConfigSerdeExt;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;

/// Caps input decoding and generated semantic work per fuzz invocation.
const MAX_INPUT_BYTES: usize = 64 * 1024;

/// Represents typed fields exercised by the structured deserialization target.
#[derive(Debug, Deserialize, PartialEq)]
struct TypedConfig {
    /// Optional signed integer, including values outside the usual machine
    /// range.
    signed: Option<i128>,
    /// Optional unsigned integer, including values beyond `u64`.
    unsigned: Option<u128>,
    /// Optional duration deserialized from seconds and nanoseconds.
    timeout: Option<Duration>,
    /// Optional enum covering unit, newtype, and record visitor shapes.
    mode: Option<Mode>,
    /// Optional newtype string value.
    label: Option<Label>,
}

/// Newtype wrapper used to exercise Serde newtype visitors.
#[derive(Debug, Deserialize, PartialEq)]
struct Label(String);

/// Enum variants exercise unit, newtype, and record deserialization forms.
#[derive(Debug, Deserialize, PartialEq)]
enum Mode {
    /// Unit variant for a fixed mode.
    Fast,
    /// Alternate unit variant.
    Slow,
    /// Newtype variant containing a label.
    Labeled(Label),
    /// Record variant with an optional byte field.
    Record { value: Option<u8> },
}

/// Builds equivalent typed fixtures with varied optional and boundary values.
fn generated_case(data: &[u8]) -> (TypedConfig, Config, Config, Config) {
    let signed_cases = [
        i128::MIN,
        i128::from(i64::MIN) - 1,
        i128::from(i64::MIN),
        -1,
        0,
        i128::from(i64::MAX),
        i128::from(i64::MAX) + 1,
        i128::MAX,
    ];
    let unsigned_cases = [0, 1, u128::from(u64::MAX), u128::from(u64::MAX) + 1, u128::MAX];
    let signed = signed_cases[usize::from(data.first().copied().unwrap_or(0)) % signed_cases.len()];
    let unsigned = unsigned_cases[usize::from(data.get(1).copied().unwrap_or(0)) % unsigned_cases.len()];
    let label: String = data.iter().take(16).map(|byte| char::from(b'a' + byte % 26)).collect();
    let mut expected = TypedConfig {
        signed: None,
        unsigned: None,
        timeout: None,
        mode: None,
        label: None,
    };
    let mut fields = Map::new();
    let states = [8_usize, 9, 10, 11, 12].map(|index| data.get(index).copied().unwrap_or(0) % 3);
    if states[0] == 0 {
        expected.signed = Some(signed);
        fields.insert("signed".into(), Value::String(signed.to_string()));
    } else if states[0] == 1 {
        fields.insert("signed".into(), Value::Null);
    }
    if states[1] == 0 {
        expected.unsigned = Some(unsigned);
        fields.insert("unsigned".into(), Value::String(unsigned.to_string()));
    } else if states[1] == 1 {
        fields.insert("unsigned".into(), Value::Null);
    }
    let duration = Duration::new(
        u64::from(data.get(4).copied().unwrap_or(0)),
        u32::from(data.get(5).copied().unwrap_or(0)),
    );
    if states[2] == 0 {
        expected.timeout = Some(duration);
        fields.insert(
            "timeout".into(),
            serde_json::json!({"secs": duration.as_secs(), "nanos": duration.subsec_nanos()}),
        );
    } else if states[2] == 1 {
        fields.insert("timeout".into(), Value::Null);
    }
    if states[3] == 0 {
        let (value, typed) = match data.get(2).copied().unwrap_or(0) % 4 {
            0 => (serde_json::json!("Fast"), Mode::Fast),
            1 => (serde_json::json!("Slow"), Mode::Slow),
            2 => (
                serde_json::json!({"Labeled": label}),
                Mode::Labeled(Label(label.clone())),
            ),
            _ => {
                let raw = data.get(3).copied().unwrap_or(0);
                let value = if raw % 2 == 0 { Some(raw) } else { None };
                (serde_json::json!({"Record": {"value": value}}), Mode::Record { value })
            }
        };
        fields.insert("mode".into(), value);
        expected.mode = Some(typed);
    } else if states[3] == 1 {
        fields.insert("mode".into(), Value::Null);
    }
    if states[4] == 0 {
        expected.label = Some(Label(label.clone()));
        fields.insert("label".into(), Value::String(label));
    } else if states[4] == 1 {
        fields.insert("label".into(), Value::Null);
    }
    if fields.is_empty() {
        fields.insert("timeout".into(), Value::Null);
    }
    let object = Value::Object(fields);

    let mut dotted = Config::new();
    for (key, value) in object.as_object().expect("generated object") {
        dotted
            .set(format!("case.{key}"), value.clone())
            .expect("set dotted field");
    }

    let mut root = Config::new();
    for (key, value) in object.as_object().expect("generated object") {
        root.set(key, value.clone()).expect("set root field");
    }

    let mut exact = Config::new();
    exact.set("case", object).expect("set exact case object");
    (expected, dotted, root, exact)
}

/// Verifies equivalent layouts deserialize identically with interpolation on
/// and off.
fn assert_generated_semantics(data: &[u8]) {
    let (expected, dotted, root, exact) = generated_case(data);
    for interpolate in [false, true] {
        let options = ConfigDeserializeOptions {
            interpolate,
            unknown_fields: UnknownFieldPolicy::Reject,
        };
        assert_eq!(
            dotted
                .deserialize_with::<TypedConfig>("case", options)
                .expect("read dotted case"),
            expected,
        );
        assert_eq!(
            dotted
                .section("case")
                .expect("select dotted section")
                .deserialize_with::<TypedConfig>("", options)
                .expect("read dotted section"),
            expected,
        );
        assert_eq!(
            root.deserialize_with::<TypedConfig>("", options)
                .expect("read root fields"),
            expected,
        );
        assert_eq!(
            exact
                .deserialize_with::<TypedConfig>("case", options)
                .expect("read exact case"),
            expected,
        );
    }
}

/// Consumes successful typed values so every representative visitor field is
/// part of the fuzzing workload.
fn consume_typed_config(config: TypedConfig) {
    let TypedConfig {
        signed,
        unsigned,
        timeout,
        mode,
        label,
    } = config;
    black_box((signed, unsigned, timeout, label));
    if let Some(mode) = mode {
        match mode {
            Mode::Fast | Mode::Slow => {}
            Mode::Labeled(Label(value)) => {
                black_box(value);
            }
            Mode::Record { value } => {
                black_box(value);
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    assert_generated_semantics(data);
    let Ok(config) = Config::decode_json_slice(data) else {
        return;
    };

    let options = ConfigDeserializeOptions {
        unknown_fields: UnknownFieldPolicy::Ignore,
        ..Default::default()
    };
    if let Ok(value) = config.deserialize_with::<TypedConfig>("", options) {
        consume_typed_config(value);
    }
    for key in config.keys().into_iter().take(8) {
        if let Ok(value) = config.deserialize_with::<TypedConfig>(&key, options) {
            consume_typed_config(value);
        }
    }
    let options = ConfigDeserializeOptions {
        interpolate: true,
        unknown_fields: UnknownFieldPolicy::Ignore,
    };
    if let Ok(value) = config.deserialize_with::<TypedConfig>("", options) {
        consume_typed_config(value);
    }
    for key in config.keys().into_iter().take(8) {
        if let Ok(value) = config.deserialize_with::<TypedConfig>(&key, options) {
            consume_typed_config(value);
        }
    }
});
