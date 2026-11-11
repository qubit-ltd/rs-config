// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for public configuration access APIs.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;

#[test]
fn test_len_reports_the_number_of_properties() {
    let mut config = Config::new();
    assert_eq!(config.len(), 0);
    config.set("server.port", 8080).unwrap();
    config.set("server.host", "localhost").unwrap();
    assert_eq!(config.len(), 2);
}

#[test]
fn test_set_description_updates_config_metadata() {
    let mut config = Config::new();

    config.set_description(Some("service config".to_string()));
    assert_eq!(config.description(), Some("service config"));

    config.set_description(None);
    assert_eq!(config.description(), None);
}

#[test]
fn test_section_if_present_reports_descendants_and_rejects_invalid_paths() {
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("scalar", "value").unwrap();

    let server = config.section_if_present("server").unwrap().unwrap();
    assert_eq!(server.get::<u16>("port").unwrap(), 8080);
    assert!(config.section_if_present("scalar").unwrap().is_none());
    assert!(config.section_if_present("missing").unwrap().is_none());
    assert!(matches!(
        config.section_if_present(".server"),
        Err(ConfigError::InvalidPath { .. })
    ));
}
