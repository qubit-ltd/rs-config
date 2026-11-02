// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes structured deserialization from validated wire configurations.

use libfuzzer_sys::fuzz_target;
use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::UnknownFieldPolicy;
use qubit_config::conversion::ConfigSerdeExt;
use serde_json::Value;

const MAX_INPUT_BYTES: usize = 64 * 1024;

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
    let (expected, dotted, root, exact) = generated_case(data);
    assert_eq!(dotted.deserialize::<Value>("case").expect("read dotted case"), expected);
    assert_eq!(
        dotted
            .section("case")
            .expect("select dotted section")
            .deserialize::<Value>("")
            .expect("read dotted section"),
        expected,
    );
    assert_eq!(root.deserialize::<Value>("").expect("read root fields"), expected);
    assert_eq!(exact.deserialize::<Value>("case").expect("read exact case"), expected);
    let interpolated = ConfigDeserializeOptions {
        interpolate: true,
        unknown_fields: UnknownFieldPolicy::Reject,
    };
    assert_eq!(
        dotted
            .deserialize_with::<Value>("case", interpolated)
            .expect("read interpolated dotted case"),
        expected,
    );
    assert_eq!(
        dotted
            .section("case")
            .expect("select dotted section")
            .deserialize_with::<Value>("", interpolated)
            .expect("read interpolated dotted section"),
        expected,
    );
    assert_eq!(
        root.deserialize_with::<Value>("", interpolated)
            .expect("read interpolated root fields"),
        expected,
    );
    assert_eq!(
        exact
            .deserialize_with::<Value>("case", interpolated)
            .expect("read interpolated exact case"),
        expected,
    );
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    assert_generated_semantics(data);
    let Ok(config) = Config::decode_json_slice(data) else {
        return;
    };
    let _ = config.deserialize::<serde_json::Value>("");
    for key in config.keys().into_iter().take(8) {
        let _ = config.deserialize::<serde_json::Value>(&key);
    }
    let options = ConfigDeserializeOptions {
        interpolate: true,
        unknown_fields: UnknownFieldPolicy::Ignore,
    };
    let _ = config.deserialize_with::<serde_json::Value>("", options);
    for key in config.keys().into_iter().take(8) {
        let _ = config.deserialize_with::<serde_json::Value>(&key, options);
    }
});
