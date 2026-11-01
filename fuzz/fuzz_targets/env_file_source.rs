// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded `.env` parsing and substitution escaping.

use libfuzzer_sys::fuzz_target;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvFileConfigSource;

const MAX_INPUT_BYTES: usize = 64 * 1024;
const PLACEHOLDER: &str = "RS_CONFIG_FUZZ_LITERAL_7D3C91";

fn hex_prefix(data: &[u8]) -> String {
    data.iter().take(64).map(|byte| format!("{byte:02x}")).collect()
}

fn comment_prefix(data: &[u8]) -> String {
    const CHARACTERS: &[u8] = b"quote'\"\\#$";

    data.iter()
        .take(64)
        .map(|byte| CHARACTERS[usize::from(*byte) % CHARACTERS.len()] as char)
        .collect()
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let payload = hex_prefix(data);
    let comment = comment_prefix(data);
    let expected = format!("{payload}-${PLACEHOLDER}-${{{PLACEHOLDER}}}");
    let assignment = match data.first().copied().unwrap_or_default() % 3 {
        0 => format!("VALUE={expected}\n"),
        1 => format!("VALUE=\"{expected}\"\n"),
        _ => format!("VALUE='{expected}'\n"),
    };
    let generated = format!("# {comment}\n{assignment}");
    let config = EnvFileConfigSource::from_content(generated)
        .load()
        .expect("generated valid dotenv must load");
    assert_eq!(config.get::<String>("VALUE").unwrap(), expected);

    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let source = EnvFileConfigSource::from_content(content);
    let _ = source.load();
});
