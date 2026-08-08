// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::conversion::ConfigSerdeExt;
use qubit_config::options::ReadPolicy;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct RetrySettings {
    max_attempts: u32,
    enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ServerSettings {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct EnvironmentSettings {
    enabled: bool,
    ports: Vec<u16>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct RawSettings {
    max_attempts: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct LabelSettings {
    label: String,
}

/// Deserializes retry settings from any supported configuration reader.
fn read_retry<R>(reader: &R) -> ConfigResult<RetrySettings>
where
    R: ConfigReader + ?Sized,
{
    reader.deserialize_interpolated("")
}

#[test]
fn test_deserialize_interpolated_supports_generic_scoped_reader() {
    let mut config = Config::new();
    config
        .set("default_attempts", "4")
        .expect("default attempt count should be set");
    config
        .set("retry.max_attempts", "${default_attempts}")
        .expect("retry attempt placeholder should be set");
    config
        .set("retry.enabled", "true")
        .expect("retry enabled value should be set");

    let settings = read_retry(&config.section("retry").unwrap())
        .expect("scoped retry settings should deserialize");

    assert_eq!(
        settings,
        RetrySettings {
            max_attempts: 4,
            enabled: true,
        }
    );
}

/// Verifies the extension and inherent methods share one behavior.
#[test]
fn test_deserialize_matches_config_inherent_method() {
    let mut config = Config::new();
    config
        .set("server.host", "localhost")
        .expect("server host should be set");
    config
        .set("server.port", 8080_u16)
        .expect("server port should be set");

    let inherent: ServerSettings = config
        .deserialize("server")
        .expect("inherent method should deserialize");
    let extension: ServerSettings =
        ConfigSerdeExt::deserialize(&config, "server")
            .expect("extension method should deserialize");

    assert_eq!(extension, inherent);
}

/// Verifies ordinary structured reads preserve placeholders.
#[test]
fn test_deserialize_does_not_interpolate_scoped_values() {
    let mut config = Config::new();
    config
        .set("retry.max_attempts", "${default_attempts}")
        .expect("retry placeholder should be set");

    let settings: RawSettings = config
        .section("retry")
        .unwrap()
        .deserialize("")
        .expect("raw scoped settings should deserialize");

    assert_eq!(settings.max_attempts, "${default_attempts}");
}

/// Verifies a section read-options override controls structured conversion.
#[test]
fn test_deserialize_uses_section_read_policy_override() {
    let mut config = Config::new();
    config
        .set("service.enabled", "yes")
        .expect("service enabled value should be set");
    config
        .set("service.ports", "8080, 8081")
        .expect("service ports should be set");
    let options = ReadPolicy::env_friendly();
    let section = config.section("service").unwrap().read_with(&options);

    let settings: EnvironmentSettings = section
        .deserialize("")
        .expect("environment-style settings should deserialize");

    assert_eq!(
        settings,
        EnvironmentSettings {
            enabled: true,
            ports: vec![8080, 8081],
        }
    );
}

/// Verifies a selected subtree takes precedence over root fallback.
#[test]
fn test_deserialize_interpolated_prefers_selected_subtree() {
    let mut config = Config::new();
    config
        .set("attempts_value", "4")
        .expect("root attempt count should be set");
    config
        .set("retry.settings.attempts_value", "5")
        .expect("scoped attempt count should be set");
    config
        .set("retry.settings.max_attempts", "${attempts_value}")
        .expect("nested attempt placeholder should be set");
    config
        .set("retry.settings.enabled", true)
        .expect("nested enabled value should be set");

    let settings: RetrySettings = config
        .section("retry")
        .unwrap()
        .deserialize_interpolated("settings")
        .expect("nested settings should deserialize");

    assert_eq!(settings.max_attempts, 5);
}

/// Verifies exact properties cannot coexist with visible descendants.
#[test]
fn test_deserialize_reports_root_relative_exact_subtree_conflict() {
    let mut config = Config::new();
    config
        .set("retry.policy", "fixed")
        .expect("exact retry policy should be set");
    config
        .set("retry.policy.mode", "strict")
        .expect("nested retry policy should be set");

    let error = config
        .section("retry")
        .unwrap()
        .deserialize::<serde_json::Value>("policy")
        .expect_err("exact property and descendants should conflict");

    assert!(matches!(
        error,
        ConfigError::KeyConflict { path, .. } if path == "retry.policy"
    ));
}

/// Verifies interpolation limits preserve their structured error and path.
#[test]
fn test_deserialize_interpolated_preserves_expansion_limit_error() {
    let mut config = Config::new();
    config
        .set("first", "4")
        .expect("first interpolation value should be set");
    config
        .set("second", "true")
        .expect("second interpolation value should be set");
    config
        .set("retry.label", "${first}-${second}")
        .expect("label placeholders should be set");
    let options = ReadPolicy::default().with_max_interpolation_expansions(1);
    let section = config.section("retry").unwrap().read_with(&options);

    let error = section
        .deserialize_interpolated::<LabelSettings>("")
        .expect_err("two placeholders should exceed the expansion limit");

    assert!(matches!(
        error,
        ConfigError::SubstitutionExpansionLimitExceeded {
            path,
            max_expansions: 1,
        } if path == "retry.label"
    ));
}
