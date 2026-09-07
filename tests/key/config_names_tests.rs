// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for configuration key list argument adapters.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigPathViolation;

#[test]
fn test_config_names_accepts_str_slice_array_and_vec() {
    let mut config = Config::new();
    config
        .set("legacy.port", 8080i32)
        .expect("setting config value should succeed");

    let slice: &[&str] = &["server.port", "legacy.port"];
    let vec_names = vec!["server.port", "legacy.port"];

    assert_eq!(config.get_any::<i32>(slice).unwrap(), 8080);
    assert_eq!(config.get_any::<i32>(["server.port", "legacy.port"]).unwrap(), 8080);
    assert_eq!(config.get_any::<i32>(&["server.port", "legacy.port"]).unwrap(), 8080);
    assert_eq!(config.get_any::<i32>(vec_names).unwrap(), 8080);
}

#[test]
fn test_config_names_accepts_owned_string_lists() {
    let mut config = Config::new();
    config
        .set("APP_HOST", "localhost")
        .expect("setting config value should succeed");

    let array = [String::from("server.host"), String::from("APP_HOST")];
    let vec_names = vec![String::from("server.host"), String::from("APP_HOST")];

    assert_eq!(config.get_any::<String>(&array).unwrap(), "localhost");
    assert_eq!(config.get_any::<String>(array).unwrap(), "localhost");
    assert_eq!(config.get_any::<String>(&vec_names).unwrap(), "localhost");
    assert_eq!(config.get_any::<String>(vec_names).unwrap(), "localhost");
}

#[test]
fn config_names_empty_inputs_have_distinct_optional_and_required_results() {
    let config = Config::new();
    let empty: &[&str] = &[];

    assert_eq!(config.get_optional_any::<String>(empty).unwrap(), None);

    let error = config.get_any::<String>(empty).unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(error.candidate_paths(), Some([].as_slice()));
}

#[test]
fn config_names_validate_every_candidate_before_searching() {
    let mut config = Config::new();
    config.set("present", 7_i32).unwrap();

    let error = config
        .get_optional_any::<i32>(["present", "bad..candidate"])
        .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::InvalidKey {
            violation: ConfigPathViolation::EmptySegment,
            ..
        }
    ));
}

#[test]
fn config_names_support_unicode_candidates_and_preserve_priority() {
    let mut config = Config::new();
    config.set("服务.备用", "fallback").unwrap();

    assert_eq!(
        config
            .get_any::<String>(["服务.主", "服务.备用"])
            .unwrap(),
        "fallback"
    );
}
