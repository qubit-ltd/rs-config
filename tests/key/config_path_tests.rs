// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for canonical configuration keys and paths.

use qubit_config::{Config, ConfigError, ConfigKey, ConfigPath, ConfigPathViolation, ConfigReader};

#[test]
fn config_key_accepts_canonical_dotted_names() {
    for value in [
        "server",
        "server.port",
        "default_headers.x-request-id",
        "服务.端口",
    ] {
        assert_eq!(ConfigKey::parse(value).unwrap().as_str(), value);
    }
}

#[test]
fn config_key_rejects_malformed_names_without_normalizing() {
    let cases = [
        ("", ConfigPathViolation::Empty),
        (".server", ConfigPathViolation::LeadingSeparator),
        ("server.", ConfigPathViolation::TrailingSeparator),
        ("server..port", ConfigPathViolation::EmptySegment),
    ];
    for (value, expected) in cases {
        assert!(matches!(
            ConfigKey::parse(value),
            Err(ConfigError::InvalidKey { violation, .. })
                if violation == expected
        ));
    }
}

#[test]
fn config_path_allows_only_the_empty_root_exception() {
    assert_eq!(ConfigPath::parse("").unwrap().as_str(), "");
    assert!(ConfigPath::parse("server.port").is_ok());
    assert!(matches!(
        ConfigPath::parse(".server"),
        Err(ConfigError::InvalidPath {
            violation: ConfigPathViolation::LeadingSeparator,
            ..
        })
    ));
}

#[test]
fn path_sensitive_lookups_reject_malformed_keys() {
    let config = Config::new();
    assert!(matches!(
        config.get_property("bad..key"),
        Err(ConfigError::InvalidKey { .. })
    ));
    assert!(matches!(
        config.contains(".server"),
        Err(ConfigError::InvalidKey { .. })
    ));
    assert!(matches!(
        config.is_unset("server."),
        Err(ConfigError::InvalidKey { .. })
    ));
}

#[test]
fn sections_reject_malformed_paths_and_preserve_the_root() {
    let config = Config::new();
    assert!(matches!(
        config.section(".http"),
        Err(ConfigError::InvalidPath { .. })
    ));
    assert_eq!(config.section("").unwrap().path(), "");
}

#[test]
fn nested_sections_validate_before_joining_paths() {
    let config = Config::new();
    let http = config.section("http").unwrap();
    let proxy = http.section("proxy").unwrap();
    assert_eq!(proxy.path(), "http.proxy");
    assert!(matches!(
        proxy.section("bad..path"),
        Err(ConfigError::InvalidPath { .. })
    ));
    assert_eq!(
        ConfigReader::resolve_key(&proxy, "host").unwrap(),
        "http.proxy.host"
    );
}

#[test]
fn multi_key_reads_validate_every_candidate_before_lookup() {
    let mut config = Config::new();
    config.set("present", 7u8).unwrap();

    assert!(matches!(
        config.get_any::<u8>(["present", "bad..candidate"]),
        Err(ConfigError::InvalidKey { .. })
    ));
}

#[test]
fn writes_and_removals_share_the_canonical_key_contract() {
    let mut config = Config::new();
    assert!(matches!(
        config.set(".bad", 1u8),
        Err(ConfigError::InvalidKey { .. })
    ));
    assert!(matches!(
        config.remove("bad."),
        Err(ConfigError::InvalidKey { .. })
    ));
}
