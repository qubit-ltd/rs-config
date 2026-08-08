// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for parsing context behavior observable through typed reads.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;

#[test]
fn test_config_parse_context_uses_resolved_key_in_errors() {
    let mut config = Config::new();
    config
        .set("http.port", "invalid")
        .expect("setting config value should succeed");

    let view = config.section("http").unwrap();
    let error = view
        .get::<u16>("port")
        .expect_err("invalid integer should fail conversion");

    assert!(matches!(
        error,
        ConfigError::ConversionError { key, .. } if key == "http.port"
    ));
}

#[test]
fn test_config_parse_context_applies_interpolation_for_explicit_reads() {
    let mut config = Config::new();
    config
        .set("http.host", "localhost")
        .expect("setting host should succeed");
    config
        .set("http.url", "http://${host}:8080")
        .expect("setting URL should succeed");

    let view = config.section("http").unwrap();
    let url = view
        .get_interpolated::<String>("url")
        .expect("interpolated read should use the view context");

    assert_eq!(url, "http://localhost:8080");
}
