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
use qubit_config::source::SourceLimitKind;
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

    let width = 1 + data.first().copied().unwrap_or(0) as usize % 16;
    let wide = (0..width).map(|i| format!("k{i}: 1\n")).collect::<String>();
    let zero_budget = YamlConfigSource::builder()
        .content(wide.clone())
        .limits(SourceLimits::builder().max_parser_tokens(0).build())
        .build()
        .load();
    assert!(matches!(
        zero_budget,
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::ParserTokenCount,
            limit: 0,
            ..
        })
    ));
    let accepted = YamlConfigSource::builder()
        .content(wide)
        .limits(SourceLimits::builder().max_parser_tokens(1024).build())
        .build()
        .load()
        .expect("bounded generated mapping should fit the generous token budget");
    assert_eq!(accepted.len(), width);
    for i in 0..width {
        assert_eq!(accepted.get::<i64>(&format!("k{i}")).unwrap(), 1);
    }

    let depth = 1 + data.get(1).copied().unwrap_or(0) as usize % 8;
    let mut deep = "leaf: 1".to_owned();
    for _ in 0..depth {
        deep = format!("k: {{{deep}}}");
    }
    let path = format!("{}.leaf", vec!["k"; depth].join("."));
    let accepted = YamlConfigSource::builder()
        .content(deep.clone())
        .limits(SourceLimits::builder().max_parser_nesting_depth(depth).build())
        .build()
        .load()
        .expect("exact generated nesting depth should be admitted");
    assert_eq!(accepted.get::<i64>(&path).unwrap(), 1);
    let rejected = YamlConfigSource::builder()
        .content(deep)
        .limits(SourceLimits::builder().max_parser_nesting_depth(depth - 1).build())
        .build()
        .load();
    assert!(matches!(
        rejected,
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::ParserNestingDepth,
            limit,
            ..
        }) if limit == depth - 1
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
