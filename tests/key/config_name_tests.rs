// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for configuration key argument adapters.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigKey;
use qubit_config::ConfigPathViolation;
use qubit_config::ConfigReader;

#[test]
fn test_config_name_accepts_standard_borrowed_text_adapters() {
    struct Key<'a>(&'a str);
    impl AsRef<str> for Key<'_> {
        fn as_ref(&self) -> &str {
            self.0
        }
    }
    let mut config = Config::new();
    config.set(Key("server.port"), 8080_i32).unwrap();
    assert_eq!(
        config.get::<i32>(std::borrow::Cow::Borrowed("server.port")).unwrap(),
        8080
    );
    assert_eq!(ConfigReader::get::<i32>(&config, Key("server.port")).unwrap(), 8080);
    let section = config.section("server").unwrap();
    assert_eq!(section.get::<i32>(Key("port")).unwrap(), 8080);
    let property = config.get_property(String::from("server.port")).unwrap().unwrap();
    assert_eq!(property.name(), "server.port");
}

#[test]
fn test_config_name_accepts_str_string_and_string_ref() {
    let mut config = Config::new();
    config
        .set("server.host", "localhost")
        .expect("setting config value should succeed");

    let owned = String::from("server.host");
    let borrowed = String::from("server.host");

    assert_eq!(config.get::<String>("server.host").unwrap(), "localhost");
    assert_eq!(config.get::<String>(owned).unwrap(), "localhost");
    assert_eq!(config.get::<String>(&borrowed).unwrap(), "localhost");
}

#[test]
fn test_config_name_resolves_relative_to_section() {
    let mut config = Config::new();
    config
        .set("http.host", "localhost")
        .expect("setting config value should succeed");

    let view = config.section("http").unwrap();
    let name = String::from("host");

    assert!(ConfigReader::contains(&view, &name).unwrap());
    assert_eq!(view.get::<String>(name).unwrap(), "localhost");
}

#[test]
fn config_name_accepts_owned_and_borrowed_config_keys_without_rewriting_unicode() {
    let key = ConfigKey::parse("服务.端口").unwrap();
    let mut config = Config::new();
    config.set(key.as_str(), 8080_i32).unwrap();

    assert_eq!(config.get::<i32>(key.clone()).unwrap(), 8080);
    assert_eq!(config.get::<i32>(&key).unwrap(), 8080);
    assert_eq!(ConfigReader::resolve_key(&config, key).unwrap(), "服务.端口");
}

#[test]
fn config_name_preserves_leading_and_trailing_spaces_as_literal_key_text() {
    let key = " server.host ";
    let parsed = ConfigKey::parse(key).unwrap();

    assert_eq!(parsed.as_str(), key);
    assert_eq!(parsed.into_string(), key);
}

#[test]
fn config_name_rejects_empty_keys_before_lookup() {
    let config = Config::new();
    let error = config.contains(String::new()).unwrap_err();

    assert!(matches!(
        error,
        ConfigError::InvalidKey {
            key,
            violation: ConfigPathViolation::Empty,
        } if key.is_empty()
    ));
}
