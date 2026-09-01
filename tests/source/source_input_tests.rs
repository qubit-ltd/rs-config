// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::SourceLimitKind;
use qubit_config::source::ConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimits;

#[test]
fn properties_source_rejects_oversized_in_memory_input_before_loading() {
    let source = PropertiesConfigSource::builder()
        .content("key=abcd\n")
        .limits(SourceLimits::builder().max_input_bytes(3).build())
        .build();
    let _config = Config::new();

    let error = source.load().expect_err("oversized in-memory input must be rejected");
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
