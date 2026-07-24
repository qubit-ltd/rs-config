// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Crate root re-exports (`lib.rs`) smoke test.

const README: &str = include_str!("../README.md");
const README_ZH_CN: &str = include_str!("../README.zh_CN.md");

#[test]
fn crate_public_api_is_reachable() {
    let _ = qubit_config::Config::new();
}

#[test]
fn crate_public_modules_are_reachable() {
    fn assert_from_config<T: qubit_config::from::FromConfig>() {}

    assert_from_config::<u16>();
    let _ = qubit_config::options::ReadOptions::default();
    let _ = qubit_config::field::ConfigField::<bool>::builder()
        .name("enabled")
        .build();
}

/// Verifies both README files use current and process-safe examples.
#[test]
fn readmes_use_current_dependency_and_safe_examples() {
    for readme in [README, README_ZH_CN] {
        assert!(readme.contains("qubit-datatype = { version = \"0.9\""));
        assert!(readme.contains("config.deserialize(\"db\")?"));
        assert!(!readme.contains("std::env::set_var"));
    }
}
