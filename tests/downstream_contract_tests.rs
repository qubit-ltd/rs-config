// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for the reader combinations used by `rs-http` and `rs-mime`.

use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::InterpolationSources;
use qubit_config::ReadPolicy;
use qubit_config::source::EnvConfigOptions;

fn env_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment lock should not be poisoned")
}

#[test]
fn http_style_section_reads_optional_and_interpolated_values() {
    let mut config = Config::new();
    config.set("http.host", "api.example.test").unwrap();
    config.set("http.base_url", "https://${host}/v1").unwrap();
    config.set("http.ipv4_only", true).unwrap();

    let http = config.section("http").unwrap();

    assert_eq!(http.get_optional::<bool>("ipv4_only").unwrap(), Some(true));
    assert_eq!(http.get_optional::<u64>("max_redirects").unwrap(), None);
    assert_eq!(
        http.get_interpolated::<String>("base_url").unwrap(),
        "https://api.example.test/v1"
    );
}

#[test]
fn http_style_prefix_iteration_reads_default_headers_in_a_section() {
    let mut config = Config::new();
    config.set("http.token", "secret-token").unwrap();
    config
        .set("http.default_headers.authorization", "Bearer ${token}")
        .unwrap();
    config
        .set("http.default_headers.user-agent", "qubit-client")
        .unwrap();
    config
        .set("http.default_headers_extra.ignored", "nope")
        .unwrap();
    config.set("http.server.host", "api.example.test").unwrap();

    let http = config.section("http").unwrap();
    let keys: Vec<_> = http
        .iter_prefix("default_headers.")
        .map(|(key, _)| key.to_owned())
        .collect();

    assert_eq!(
        keys,
        [
            "default_headers.authorization",
            "default_headers.user-agent"
        ]
    );
    assert_eq!(
        http.get_interpolated::<String>("default_headers.authorization")
            .unwrap(),
        "Bearer secret-token"
    );
    assert!(
        !keys
            .iter()
            .any(|key| key.starts_with("default_headers_extra."))
    );
}

#[test]
fn mime_style_multi_key_interpolated_lookup_honors_priority_and_default() {
    let mut config = Config::new();
    config.set("selector", "repository").unwrap();
    config.set("mime.detector.default", "${selector}").unwrap();
    config
        .set("QUBIT_MIME_DETECTOR_DEFAULT", "environment")
        .unwrap();

    let policy = ReadPolicy::builder()
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let value_config = config.read_with(&policy);

    assert_eq!(
        value_config
            .get_any_interpolated_or::<String>(
                ["mime.detector.default", "QUBIT_MIME_DETECTOR_DEFAULT"],
                "fallback",
            )
            .unwrap(),
        "repository"
    );
    assert_eq!(
        value_config
            .get_any_interpolated_or::<String>(["missing.primary", "missing.fallback"], "fallback")
            .unwrap(),
        "fallback"
    );
}

#[test]
fn read_with_allows_explicit_environment_interpolation_for_downstream_readers() {
    let _guard = env_test_lock();
    let env_name = format!("QUBIT_RS_CONFIG_DOWNSTREAM_ENV_{}", std::process::id());
    unsafe {
        std::env::set_var(&env_name, "Bearer environment-token");
    }

    let mut config = Config::new();
    config
        .set(
            "http.default_headers.authorization",
            format!("${{{env_name}}}"),
        )
        .unwrap();

    let default_error = config
        .get_interpolated::<String>("http.default_headers.authorization")
        .expect_err("default reader must not consult the process environment");
    assert_eq!(default_error.kind(), ConfigErrorKind::Substitution);

    let policy = ReadPolicy::builder()
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let env_view = config.read_with(&policy);
    assert_eq!(
        env_view
            .get_any_interpolated_or::<String>(["http.default_headers.authorization"], "fallback")
            .unwrap(),
        "Bearer environment-token"
    );

    unsafe {
        std::env::remove_var(&env_name);
    }
}

#[test]
fn mime_style_env_config_options_load_prefixed_keys_without_normalization() {
    let _guard = env_test_lock();
    let suffix = std::process::id();
    let matching = format!("QUBIT_RS_CONFIG_DOWNSTREAM_OPTION_{suffix}");
    let unrelated = format!("RS_CONFIG_DOWNSTREAM_UNRELATED_{suffix}");
    unsafe {
        std::env::set_var(&matching, "repository");
        std::env::set_var(&unrelated, "must-not-load");
    }

    let options = EnvConfigOptions::builder()
        .prefix("QUBIT_RS_CONFIG_DOWNSTREAM_")
        .build();
    let config = Config::from_env_options(options).unwrap();

    assert_eq!(config.get::<String>(&matching).unwrap(), "repository");
    assert!(!config.contains(&unrelated).unwrap());

    unsafe {
        std::env::remove_var(&matching);
        std::env::remove_var(&unrelated);
    }
}

#[test]
fn downstream_missing_and_conversion_failures_keep_stable_error_kinds() {
    let mut config = Config::new();
    let missing = config
        .section("http")
        .unwrap()
        .get_any::<String>(["base_url", "QUBIT_HTTP_BASE_URL"])
        .expect_err("missing HTTP candidates should be reported");

    assert_eq!(missing.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(
        missing.candidate_paths(),
        Some(
            [
                "http.base_url".to_string(),
                "http.QUBIT_HTTP_BASE_URL".to_string()
            ]
            .as_slice()
        )
    );

    config
        .set("mime.enable.precise.detection", "not-a-boolean")
        .unwrap();
    let conversion = config
        .get_any_interpolated_or(
            [
                "mime.enable.precise.detection",
                "QUBIT_MIME_ENABLE_PRECISE_DETECTION",
            ],
            true,
        )
        .expect_err("an invalid MIME boolean must not be hidden by the default");
    assert_eq!(conversion.kind(), ConfigErrorKind::Conversion);
}
