// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded legacy and V1 `Config` JSON persistence decoding.

use libfuzzer_sys::fuzz_target;
use qubit_config::Config;

/// Bounds JSON parsing and nested wire allocation while retaining useful input.
const MAX_JSON_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_JSON_BYTES {
        return;
    }

    let Ok(config) = Config::decode_json_slice(data) else {
        return;
    };
    let encoded = serde_json::to_vec(&config)
        .expect("a decoded config must serialize through the V1 wire format");
    let decoded =
        Config::decode_json_slice(&encoded).expect("a serialized V1 config must deserialize");

    assert_eq!(decoded, config);
});
