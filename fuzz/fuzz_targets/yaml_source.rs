// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded YAML parsing and flattening.

use libfuzzer_sys::fuzz_target;
use qubit_config::ConfigError;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::YamlConfigSource;

const MAX_INPUT_BYTES: usize = 64 * 1024;

fn hex_prefix(data: &[u8]) -> String {
    data.iter().take(64).map(|byte| format!("{byte:02x}")).collect()
}

fn marker_text(data: &[u8]) -> String {
    const CHARACTERS: &[u8] = b"*&#:\"' \\";

    data.iter()
        .take(64)
        .map(|byte| CHARACTERS[usize::from(*byte) % CHARACTERS.len()] as char)
        .collect()
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let text = marker_text(data);
    let quoted = serde_json::to_string(&text).expect("generated text should serialize");
    let safe = YamlConfigSource::from_content(format!("message: {quoted}\n"))
        .load()
        .expect("generated quoted YAML scalar must load");
    assert_eq!(safe.get::<String>("message").unwrap(), text);

    let anchor = format!("a{}", hex_prefix(data));
    let with_alias = format!("first: &{anchor} hello\nsecond: *{anchor}\n");
    assert!(matches!(
        YamlConfigSource::from_content(with_alias).load(),
        Err(ConfigError::SourceParseError { .. })
    ));

    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let source = YamlConfigSource::builder()
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
