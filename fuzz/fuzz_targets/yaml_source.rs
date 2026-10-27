// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded YAML parsing and flattening.

use std::sync::Once;

use libfuzzer_sys::fuzz_target;
use qubit_config::source::ConfigSource;
use qubit_config::source::YamlConfigSource;

const MAX_INPUT_BYTES: usize = 64 * 1024;
static CHECK_ORACLE: Once = Once::new();

fuzz_target!(|data: &[u8]| {
    CHECK_ORACLE.call_once(|| {
        for input in [
            "note: don't expand\nfirst: &a hello\nsecond: *a\n",
            "first: &-a hello\nsecond: *-a\n",
        ] {
            assert!(YamlConfigSource::from_content(input).load().is_err());
        }

        let safe = YamlConfigSource::from_content("message: 'text with * and & markers'\n")
            .load()
            .expect("ordinary scalar marker characters should remain accepted");
        assert_eq!(safe.get::<String>("message").unwrap(), "text with * and & markers");
    });

    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let source = YamlConfigSource::from_content(content);
    let _ = source.load();
});
