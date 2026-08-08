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
use serde::Deserialize;

const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
struct TypedConfig {
    signed: Option<i128>,
    unsigned: Option<u128>,
    timeout: Option<Duration>,
    mode: Option<Mode>,
    label: Option<Label>,
}

#[derive(Debug, Deserialize)]
struct Label(String);

#[derive(Debug, Deserialize)]
enum Mode {
    Fast,
    Slow,
    Labeled(Label),
    Record { value: Option<u8> },
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
    let Ok(config) = Config::decode_json_slice(data) else {
        return;
    };

    if let Ok(value) = config.deserialize_lenient::<TypedConfig>("") {
        consume_typed_config(value);
    }
    for key in config.keys().into_iter().take(8) {
        if let Ok(value) = config.deserialize_lenient::<TypedConfig>(&key) {
            consume_typed_config(value);
        }
    }
});
