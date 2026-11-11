// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Smoke tests for crate-root public API reachability and README examples.

use qubit_config::Config;
use qubit_config::conversion::FromConfig;
use qubit_config::options::InterpolationSources;
use qubit_config::options::ReadPolicy;
use qubit_config::source::EnvConfigOptions;

const README: &str = include_str!("../README.md");
const README_ZH_CN: &str = include_str!("../README.zh_CN.md");

#[test]
fn test_crate_public_api_is_reachable() {
    let _ = Config::new();
}

#[test]
fn test_crate_public_modules_are_reachable() {
    fn assert_from_config<T: FromConfig>() {}

    assert_from_config::<u16>();
    let _ = ReadPolicy::default();
    let _ = InterpolationSources::ConfigThenEnv;
    let _ = EnvConfigOptions::builder().build();
}

/// Verifies both README files use current and process-safe examples.
#[test]
fn test_readmes_use_current_dependency_and_safe_examples() {
    for (name, readme) in [("README.md", README), ("README.zh_CN.md", README_ZH_CN)] {
        assert!(
            readme.contains("qubit-datatype = { version = \"0.15\""),
            "{name} must use the current qubit-datatype dependency"
        );
        assert!(
            readme.contains(".deserialize::<Database>(\"db\")"),
            "{name} must use the current database deserialization example"
        );
        assert!(
            readme.contains("config.deserialize_with::<Database>("),
            "{name} must show deserialization with the current config API"
        );
        assert!(
            readme.contains("unknown_fields: UnknownFieldPolicy::Ignore"),
            "{name} must document the explicit unknown-field policy"
        );
        assert!(
            !readme.contains("std::env::set_var"),
            "{name} must avoid process-global environment mutation"
        );
    }
}
