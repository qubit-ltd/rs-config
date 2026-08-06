// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::{
    Config,
    ConfigError,
    ConfigResult,
    SourceLimitKind,
    source::{
        ConfigSource,
        PropertiesConfigSource,
        SourceLimits,
    },
};

#[test]
fn properties_source_rejects_oversized_in_memory_input_before_loading() {
    let source = PropertiesConfigSource::from_content("key=abcd\n")
        .with_limits(SourceLimits::default().with_max_input_bytes(3));
    let mut config = Config::new();

    let error = source
        .load()
        .expect_err("oversized in-memory input must be rejected");
    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::InputBytes,
            limit: 3,
            observed_at_least: 9,
            ..
        }
    ));
}
