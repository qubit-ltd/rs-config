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
use serde_json::Value;

const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize, PartialEq)]
struct TypedConfig {
    signed: Option<i128>,
    unsigned: Option<u128>,
    timeout: Option<Duration>,
    mode: Option<Mode>,
    label: Option<Label>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Label(String);

#[derive(Debug, Deserialize, PartialEq)]
enum Mode {
    Fast,
    Slow,
    Labeled(Label),
    Record { value: Option<u8> },
}

fn generated_case(data: &[u8]) -> (Value, Config, Config, Config) {
    let seed = data.first().copied().unwrap_or(0);
    let signed = i64::from(seed) - 128;
    let unsigned = u64::from(seed);
    let mode = if seed % 2 == 0 { "Fast" } else { "Slow" };
    let label: String = data.iter().take(16).map(|byte| char::from(b'a' + byte % 26)).collect();
    let expected = serde_json::json!({
        "signed": signed,
        "unsigned": unsigned,
        "timeout": {"secs": unsigned, "nanos": 0},
        "mode": mode,
        "label": label,
    });

    let mut dotted = Config::new();
    dotted.set("case.signed", signed).expect("set signed value");
    dotted.set("case.unsigned", unsigned).expect("set unsigned value");
    dotted
        .set("case.timeout", expected["timeout"].clone())
        .expect("set timeout object");
    dotted.set("case.mode", mode).expect("set mode value");
    dotted.set("case.label", label.as_str()).expect("set label value");

    let mut root = Config::new();
    root.set("signed", signed).expect("set root signed value");
    root.set("unsigned", unsigned).expect("set root unsigned value");
    root.set("timeout", expected["timeout"].clone())
        .expect("set root timeout object");
    root.set("mode", mode).expect("set root mode value");
    root.set("label", label.as_str()).expect("set root label value");

    let mut exact = Config::new();
    exact.set("case", expected.clone()).expect("set exact case object");
    (expected, dotted, root, exact)
}

fn assert_generated_semantics(data: &[u8]) {
    let (expected_value, dotted, root, exact) = generated_case(data);
    let expected: TypedConfig = serde_json::from_value(expected_value).expect("build typed expectation");
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
