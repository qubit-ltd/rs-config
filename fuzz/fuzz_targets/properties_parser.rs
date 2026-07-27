// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded Java-properties parsing through the public source API.

use libfuzzer_sys::fuzz_target;
use qubit_config::{
    Config,
    source::PropertiesConfigSource,
};

/// Limits parser work while preserving multiline and Unicode coverage.
const MAX_INPUT_BYTES: usize = 32 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };

    let entries = PropertiesConfigSource::parse_content(content);
    let mut config = Config::new();
    for (key, value) in entries {
        let _ = config.set(key, value);
    }
});
