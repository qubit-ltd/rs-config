// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded explicit interpolation without process-environment input.

use libfuzzer_sys::fuzz_target;
use qubit_config::Config;
use qubit_config::options::ReadPolicy;

/// Bounds interpolation source length before configuration allocation.
const MAX_INPUT_BYTES: usize = 32 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(value) = std::str::from_utf8(data) else {
        return;
    };

    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::config_only())
        .build();
    config
        .set("value", value)
        .expect("the fixed fuzzing key should always be valid");
    let _ = config.get_interpolated::<String>("value");
});
