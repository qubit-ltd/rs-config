// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for canonical configuration keys and paths.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigKey;
use qubit_config::ConfigPath;
use qubit_config::ConfigPathViolation;
use qubit_config::ConfigReader;

#[test]
fn config_key_accepts_canonical_dotted_names() {
    for value in ["server", "server.port", "default_headers.x-request-id", "服务.端口"] {
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
    assert!(matches!(config.section(".http"), Err(ConfigError::InvalidPath { .. })));
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
    assert_eq!(ConfigReader::resolve_key(&proxy, "host").unwrap(), "http.proxy.host");
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
    assert!(matches!(config.set(".bad", 1u8), Err(ConfigError::InvalidKey { .. })));
    assert!(matches!(config.remove("bad."), Err(ConfigError::InvalidKey { .. })));
}

#[test]
fn config_paths_preserve_unicode_and_accept_deep_valid_paths() {
    let deep_path = (0..64).map(|index| format!("层{index}")).collect::<Vec<_>>().join(".");
    let parsed = ConfigPath::parse(&deep_path).unwrap();

    assert_eq!(parsed.as_str(), deep_path);
    assert_eq!(parsed.into_string(), deep_path);
    assert_eq!(ConfigPath::parse("设置.网络.端口").unwrap().as_str(), "设置.网络.端口");
}

#[test]
fn config_path_rejects_each_separator_boundary_with_the_specific_violation() {
    for (path, violation) in [
        (".server", ConfigPathViolation::LeadingSeparator),
        ("server.", ConfigPathViolation::TrailingSeparator),
        ("server..port", ConfigPathViolation::EmptySegment),
    ] {
        let error = ConfigPath::parse(path).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::InvalidPath { violation: actual, .. } if actual == violation
        ));
    }
}

#[test]
fn prefix_queries_use_section_boundaries_and_do_not_match_siblings() {
    let mut config = Config::new();
    config.set("http.host", "localhost").unwrap();
    config.set("httpish.host", "not-http").unwrap();
    config.set("http2.host", "also-not-http").unwrap();

    let keys = config.iter_prefix("http.").map(|(key, _)| key).collect::<Vec<_>>();
    assert_eq!(keys, vec!["http.host"]);
    assert!(config.contains_section("http").unwrap());
    assert!(!config.contains_section("htt").unwrap());
}

#[test]
fn root_and_nested_paths_keep_empty_path_semantics_explicit() {
    let mut config = Config::new();
    assert!(!config.contains_section("").unwrap());
    config.set("deep.value", 1_i32).unwrap();

    let root = config.section("").unwrap();
    assert_eq!(root.path(), "");
    assert_eq!(ConfigReader::resolve_key(&root, "").unwrap(), "");

    let deep = config.section("deep").unwrap();
    assert_eq!(ConfigReader::resolve_key(&deep, "").unwrap(), "deep");
    assert_eq!(deep.get::<i32>("value").unwrap(), 1);
}

#[test]
fn path_error_retains_the_rejected_text() {
    let error = ConfigPath::parse("a..b").unwrap_err();
    assert!(matches!(
        error,
        ConfigError::InvalidPath { path, violation: ConfigPathViolation::EmptySegment }
            if path == "a..b"
    ));
}

#[test]
fn key_and_path_wrapper_traits_preserve_the_validated_text() {
    let key: ConfigKey = serde_json::from_str("\"server.port\"").unwrap();
    let key_as_str: for<'a> fn(&'a ConfigKey) -> &'a str = ConfigKey::as_str;
    let key_as_ref: for<'a> fn(&'a ConfigKey) -> &'a str = <ConfigKey as AsRef<str>>::as_ref;
    assert_eq!(std::hint::black_box(key_as_str)(&key), "server.port");
    assert_eq!(std::hint::black_box(key_as_ref)(&key), "server.port");
    assert_eq!(key.to_string(), "server.port");

    let path: ConfigPath = serde_json::from_str("\"server.http\"").unwrap();
    let path_as_str: for<'a> fn(&'a ConfigPath) -> &'a str = ConfigPath::as_str;
    let path_as_ref: for<'a> fn(&'a ConfigPath) -> &'a str = <ConfigPath as AsRef<str>>::as_ref;
    assert_eq!(std::hint::black_box(path_as_str)(&path), "server.http");
    assert_eq!(std::hint::black_box(path_as_ref)(&path), "server.http");
    assert_eq!(path.to_string(), "server.http");
}
