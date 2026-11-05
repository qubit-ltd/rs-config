// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded TOML parsing and flattening.

use libfuzzer_sys::fuzz_target;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::TomlConfigSource;

/// Caps text decoding before TOML parser work begins.
const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let source = TomlConfigSource::builder()
        .content(content)
        .limits(
            SourceLimits::builder()
                .max_parser_tokens(512)
                .max_parser_nesting_depth(32)
                .build(),
        )
        .build();
    let _ = source.load();
});
