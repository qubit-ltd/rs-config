// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for variable-substitution policy and resource limits.

use qubit_config::options::VariableSubstitutionOptions;

#[test]
fn test_default_enables_bounded_config_substitution() {
    let options = VariableSubstitutionOptions::default();

    assert!(options.is_enabled());
    assert!(!options.is_environment_fallback_enabled());
    assert_eq!(options.max_depth(), 64);
    assert_eq!(options.max_expansions(), 4_096);
    assert_eq!(options.max_output_bytes(), 1_048_576);
}

#[test]
fn test_with_methods_replace_each_substitution_policy() {
    let options = VariableSubstitutionOptions::default()
        .with_enabled(false)
        .with_environment_fallback_enabled(true)
        .with_max_depth(8)
        .with_max_expansions(16)
        .with_max_output_bytes(32);

    assert!(!options.is_enabled());
    assert!(options.is_environment_fallback_enabled());
    assert_eq!(options.max_depth(), 8);
    assert_eq!(options.max_expansions(), 16);
    assert_eq!(options.max_output_bytes(), 32);
}
