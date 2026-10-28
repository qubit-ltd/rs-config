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
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::Property;
use qubit_config::conversion::ConfigParseContext;
use qubit_config::conversion::FromConfig;

#[derive(Debug, PartialEq)]
struct ServicePort(u16);

impl FromConfig for ServicePort {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        u16::from_config(property, ctx).map(Self)
    }
}

#[test]
fn test_custom_from_config_preserves_read_context() {
    let mut config = Config::new();
    config.set("server.port", "8080").unwrap();
    assert_eq!(config.get::<ServicePort>("server.port").unwrap(), ServicePort(8080));

    config.set("server.default_port", "9090").unwrap();
    config.set("server.port", "${default_port}").unwrap();
    let server = config.section("server").unwrap();
    assert_eq!(
        server.get_interpolated::<ServicePort>("port").unwrap(),
        ServicePort(9090),
    );
    assert!(server.get::<ServicePort>("port").is_err());

    config.set("server.port", "invalid").unwrap();
    let error = config.get::<ServicePort>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.port"));
}

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
