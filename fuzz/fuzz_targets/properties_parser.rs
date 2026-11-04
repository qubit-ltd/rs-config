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
use qubit_config::source::PropertiesConfigSource;

#[path = "../../tests/support/source_semantics/properties.rs"]
mod semantics;

/// Limits parser work while preserving multiline and Unicode coverage.
const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    semantics::check(data);
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };

    let _ = PropertiesConfigSource::parse_content(content);
});
