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

const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(config) = serde_json::from_slice::<Config>(data) else {
        return;
    };
    let _ = config.deserialize::<serde_json::Value>("");
    for key in config.keys().into_iter().take(8) {
        let _ = config.deserialize::<serde_json::Value>(&key);
    }
});
