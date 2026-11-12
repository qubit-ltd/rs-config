// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Bounded semantic inputs shared by dotenv fuzzing and integration tests.

use qubit_config::source::ConfigSource;
use qubit_config::source::EnvFileConfigSource;

/// Checks a bounded, valid dotenv input selected by arbitrary bytes.
pub fn check(data: &[u8]) {
    const PLACEHOLDER: &str = "${RS_CONFIG_FUZZ_LITERAL_7D3C91}";

    let selector = data.first().copied().unwrap_or_default() % 8;
    let payload = data
        .iter()
        .skip(1)
        .take(64)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    let (input, expected) = match selector {
        // Unquoted placeholders stay literal.
        0 => (
            format!("VALUE={payload}{PLACEHOLDER}\nAFTER=sentinel\n"),
            format!("{payload}{PLACEHOLDER}"),
        ),
        // Actual LF inside a double-quoted value is retained in VALUE.
        1 => (
            format!("VALUE=\"line\n{payload}{PLACEHOLDER}\"\nAFTER=sentinel\n"),
            format!("line\n{payload}{PLACEHOLDER}"),
        ),
        // CRLF inside a double-quoted value is retained distinctly from LF.
        2 => (
            format!("VALUE=\"line\r\n{payload}{PLACEHOLDER}\"\r\nAFTER=sentinel\r\n"),
            format!("line\r\n{payload}{PLACEHOLDER}"),
        ),
        // Actual LF inside a single-quoted value is retained in VALUE.
        3 => (
            format!("VALUE='line\n{payload}{PLACEHOLDER}'\nAFTER=sentinel\n"),
            format!("line\n{payload}{PLACEHOLDER}"),
        ),
        // CRLF inside a single-quoted value is retained distinctly from LF.
        4 => (
            format!("VALUE='line\r\n{payload}{PLACEHOLDER}'\r\nAFTER=sentinel\r\n"),
            format!("line\r\n{payload}{PLACEHOLDER}"),
        ),
        // Escaped quote and paired backslashes exercise dotenv's value escapes.
        5 => (
            format!("VALUE=\"left \\\"{payload}\\\\ {PLACEHOLDER}\"\nAFTER=sentinel\n"),
            format!("left \"{payload}\\ {PLACEHOLDER}"),
        ),
        // Escaped whitespace is data, and the adjacent placeholder remains
        // literal.
        6 => (
            format!("VALUE=escaped\\ {payload}{PLACEHOLDER}\nAFTER=sentinel\n"),
            format!("escaped {payload}{PLACEHOLDER}"),
        ),
        // A trailing comment is ignored; # inside the quoted value is retained.
        _ => (
            format!("# quote'\"\\#$\nVALUE='{payload}{PLACEHOLDER}#text' # ignored\nAFTER=sentinel\n"),
            format!("{payload}{PLACEHOLDER}#text"),
        ),
    };

    let config = EnvFileConfigSource::from_content(input)
        .load()
        .expect("generated valid dotenv input should load");
    assert_eq!(config.get::<String>("VALUE").unwrap(), expected);
    assert_eq!(config.get::<String>("AFTER").unwrap(), "sentinel");
    assert_eq!(config.keys(), vec!["AFTER".to_owned(), "VALUE".to_owned()]);
}
