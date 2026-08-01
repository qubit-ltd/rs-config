// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::source::{SourceInput, SourceLimits};

#[test]
fn in_memory_input_is_rejected_before_copying_when_over_limit() {
    let input = SourceInput::Content("abcd".to_owned());
    let limits = SourceLimits::default().with_max_input_bytes(3);

    let error = input
        .read_to_string("properties", limits)
        .expect_err("oversized in-memory input must be rejected");
    assert!(matches!(
        error,
        qubit_config::ConfigError::SourceLimitExceeded {
            kind: qubit_config::SourceLimitKind::InputBytes,
            limit: 3,
            observed_at_least: 4,
            ..
        }
    ));
}
