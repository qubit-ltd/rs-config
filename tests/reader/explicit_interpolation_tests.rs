// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for explicit configuration interpolation.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Endpoint {
    url: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct RawEndpoint {
    url: String,
}

#[test]
fn test_get_preserves_placeholder() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("url", "http://${host}/api").expect("set URL");

    let value = config.get::<String>("url").expect("read URL");

    assert_eq!(value, "http://${host}/api");
}

#[test]
fn test_get_optional_and_get_any_preserve_placeholders() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("URL", "http://${host}/api").expect("set URL");

    let optional = config.get_optional::<String>("URL").expect("read optional URL");
    let any = config.get_any::<String>(["url", "URL"]).expect("read aliased URL");

    assert_eq!(optional.as_deref(), Some("http://${host}/api"));
    assert_eq!(any, "http://${host}/api");
}

#[test]
fn test_get_or_uses_default_only_when_value_is_missing() {
    let mut config = Config::new();
    config.set("port", "invalid").expect("set port");

    let missing = config
        .get_or::<u16>("missing", 8080)
        .expect("missing port should use default");
    let invalid = config.get_or::<u16>("port", 8080);

    assert_eq!(missing, 8080);
    assert!(matches!(
        invalid,
        Err(ConfigError::ConversionError { key, .. }) if key == "port"
    ));
}

#[test]
fn test_get_interpolated_resolves_string_and_numeric_values() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("port", "8080").expect("set port");
    config.set("url", "http://${host}:${port}/api").expect("set URL");
    config.set("server.port", "${port}").expect("set server port");

    let url = config.get_interpolated::<String>("url").expect("interpolate URL");
    let port = config
        .get_interpolated::<u16>("server.port")
        .expect("interpolate numeric port");

    assert_eq!(url, "http://localhost:8080/api");
    assert_eq!(port, 8080);
}

/// Verifies interpolation preserves empty placeholders while resolving later
/// valid placeholders in the same string.
#[test]
fn test_interpolation_skips_empty_placeholder_and_resolves_following_value() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config
        .set("url", "${}-${host}")
        .expect("set URL with empty placeholder");

    let value = config.get_interpolated::<String>("url").expect("interpolate URL");

    assert_eq!(value, "${}-localhost");
}

/// Verifies scoped interpolated reads fall back to root configuration keys.
#[test]
fn test_section_get_interpolated_falls_back_to_root_configuration() {
    let mut config = Config::new();
    config
        .set("global.host", "api.example.test")
        .expect("root host should be set");
    config
        .set("service.url", "https://${global.host}/v1")
        .expect("scoped URL should be set");

    let service = config.section("service").expect("service section should be valid");
    let url = service
        .get_interpolated::<String>("url")
        .expect("scoped interpolation should fall back to root keys");

    assert_eq!(url, "https://api.example.test/v1");
}

#[test]
fn test_get_optional_interpolated_and_or_handle_missing_values() {
    let config = Config::new();

    let optional = config
        .get_optional_interpolated::<String>("missing")
        .expect("read missing interpolated value");
    let default = config
        .get_interpolated_or::<u16>("missing", 8080)
        .expect("read interpolated default");

    assert_eq!(optional, None);
    assert_eq!(default, 8080);
}

#[test]
fn test_get_any_interpolated_uses_alias_order() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("URL", "http://${host}/api").expect("set URL");

    let value = config
        .get_any_interpolated::<String>(["url", "URL"])
        .expect("read interpolated alias");

    assert_eq!(value, "http://localhost/api");
}

#[test]
fn test_get_optional_any_interpolated_and_or_handle_missing_values() {
    let config = Config::new();

    let optional = config
        .get_optional_any_interpolated::<String>(["url", "URL"])
        .expect("read missing interpolated aliases");
    let default = config
        .get_any_interpolated_or::<u16>(["port", "PORT"], 8080)
        .expect("read interpolated alias default");

    assert_eq!(optional, None);
    assert_eq!(default, 8080);
}

#[test]
fn test_deserialize_is_raw_and_deserialize_interpolated_is_explicit() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("base_port", "8080").expect("set base port");
    config.set("raw.url", "http://${host}/api").expect("set raw URL");
    config
        .set("endpoint.url", "http://${host}/api")
        .expect("set endpoint URL");
    config.set("endpoint.port", "${base_port}").expect("set endpoint port");

    let raw = config
        .deserialize::<RawEndpoint>("raw")
        .expect("deserialize raw endpoint");
    let interpolated = config
        .deserialize_interpolated::<Endpoint>("endpoint")
        .expect("deserialize interpolated endpoint");

    assert_eq!(raw.url, "http://${host}/api");
    assert_eq!(
        interpolated,
        Endpoint {
            url: "http://localhost/api".to_string(),
            port: 8080,
        },
    );
}
