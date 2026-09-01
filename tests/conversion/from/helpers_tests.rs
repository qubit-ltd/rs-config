// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for read helper behavior through public APIs.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;

#[test]
fn test_helpers_treat_blank_string_as_missing_when_policy_allows() {
    let mut config = Config::new();
    config
        .set_default_read_policy(
            ReadPolicy::builder()
                .blank_string_policy(BlankStringPolicy::TreatAsMissing)
                .build(),
        )
        .set("server.host", "   ")
        .expect("setting blank value should succeed");

    assert_eq!(config.get_optional::<String>("server.host").unwrap(), None);
    assert!(matches!(
        config.get::<String>("server.host"),
        Err(ConfigError::PropertyHasNoValue(key)) if key == "server.host"
    ));
}

#[test]
fn test_interpolated_helpers_resolve_values_before_missing_check() {
    let mut config = Config::new();
    config
        .set_default_read_policy(
            ReadPolicy::builder()
                .blank_string_policy(BlankStringPolicy::TreatAsMissing)
                .build(),
        )
        .set("empty", "   ")
        .expect("setting blank value should succeed");
    config
        .set("server.host", "${empty}")
        .expect("setting substituted value should succeed");

    assert_eq!(
        config
            .get_optional_interpolated::<String>("server.host")
            .unwrap(),
        None,
    );
}
