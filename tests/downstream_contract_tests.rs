// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for the reader combinations used by `rs-http` and `rs-mime`.

use std::ffi::OsString;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::time::Duration;

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::options::InterpolationSources;
use qubit_config::options::ReadPolicy;
use qubit_config::source::EnvConfigOptions;
use qubit_datatype::CollectionConversionPolicy;
use qubit_datatype::ConversionPolicy;
use qubit_datatype::DataType;
use qubit_datatype::DurationConversionPolicy;

fn env_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment lock should not be poisoned")
}

struct EnvVarGuard {
    name: String,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(name: &str, value: &str) -> Self {
        let original = std::env::var_os(name);
        unsafe {
            std::env::set_var(name, value);
        }
        Self {
            name: name.to_owned(),
            original,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.original {
                Some(value) => std::env::set_var(&self.name, value),
                None => std::env::remove_var(&self.name),
            }
        }
    }
}

#[test]
fn http_style_section_reads_optional_and_interpolated_values() {
    let mut config = Config::new();
    config.set("http.host", "api.example.test").unwrap();
    config.set("http.base_url", "https://${host}/v1").unwrap();
    config.set("http.user_agent", "${host}").unwrap();
    config
        .set("http.log_redaction.sensitive_headers", vec!["authorization", "${host}"])
        .unwrap();
    config.set("http.ipv4_only", true).unwrap();

    let http = config.section("http").unwrap();

    assert_eq!(http.get_optional::<bool>("ipv4_only").unwrap(), Some(true));
    assert_eq!(http.get_optional::<u64>("max_redirects").unwrap(), None);
    assert_eq!(
        http.get_interpolated::<String>("base_url").unwrap(),
        "https://api.example.test/v1"
    );
    assert_eq!(
        http.get_optional_interpolated::<String>("user_agent").unwrap(),
        Some("api.example.test".to_owned())
    );
    assert_eq!(
        http.get_optional_interpolated::<Vec<String>>("log_redaction.sensitive_headers")
            .unwrap(),
        Some(vec!["authorization".to_owned(), "api.example.test".to_owned()])
    );
}

#[test]
fn http_style_prefix_iteration_reads_default_headers_in_a_section() {
    let mut config = Config::new();
    config.set("http.token", "secret-token").unwrap();
    config
        .set("http.default_headers.authorization", "Bearer ${token}")
        .unwrap();
    config.set("http.default_headers.user-agent", "qubit-client").unwrap();
    config.set("http.default_headers_extra.ignored", "nope").unwrap();
    config.set("http.server.host", "api.example.test").unwrap();

    let http = config.section("http").unwrap();
    let keys: Vec<_> = http
        .iter_prefix("default_headers.")
        .map(|(key, _)| key.to_owned())
        .collect();

    let mut sorted_keys = keys.clone();
    sorted_keys.sort_unstable();
    assert_eq!(
        sorted_keys,
        ["default_headers.authorization", "default_headers.user-agent"]
    );
    assert_eq!(
        http.get_interpolated::<String>("default_headers.authorization")
            .unwrap(),
        "Bearer secret-token"
    );
    assert!(!keys.iter().any(|key| key.starts_with("default_headers_extra.")));

    let all_keys: Vec<_> = http.iter().map(|(key, _)| key.to_owned()).collect();
    assert!(all_keys.contains(&"default_headers.authorization".to_owned()));
    assert!(all_keys.contains(&"server.host".to_owned()));
}

#[test]
fn mime_style_multi_key_interpolated_lookup_honors_priority_and_default() {
    let mut config = Config::new();
    config.set("selector", "repository").unwrap();
    config.set("mime.detector.default", "${selector}").unwrap();
    config.set("QUBIT_MIME_DETECTOR_DEFAULT", "environment").unwrap();
    config.set("mime.enable.precise.detection", "yes").unwrap();

    let policy = ReadPolicy::builder()
        .conversion_policy(ConversionPolicy::env_friendly())
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let value_config = config.read_with(&policy);

    assert_eq!(
        value_config
            .get_any_interpolated_or::<String>(["mime.detector.default", "QUBIT_MIME_DETECTOR_DEFAULT"], "fallback",)
            .unwrap(),
        "repository"
    );
    assert_eq!(
        value_config
            .get_any_interpolated_or::<String>(["missing.primary", "missing.fallback"], "fallback")
            .unwrap(),
        "fallback"
    );
    assert!(
        value_config
            .get_any_interpolated_or::<bool>(
                ["mime.enable.precise.detection", "QUBIT_MIME_ENABLE_PRECISE_DETECTION",],
                false,
            )
            .unwrap()
    );
    let strict_error = config
        .read_with(&ReadPolicy::config_only())
        .get::<bool>("mime.enable.precise.detection")
        .expect_err("the strict default boolean policy must reject MIME's yes literal");
    assert_eq!(strict_error.kind(), ConfigErrorKind::Conversion);
}

#[test]
fn mime_style_read_policies_cover_list_mapping_and_duration_values() {
    let mut config = Config::new();
    config.set("mime.detector.fallbacks", "file;repository").unwrap();
    config
        .set(
            "mime.ambiguous.mime.mapping",
            "webm:video/webm,audio/webm;ogg:video/ogg,audio/ogg",
        )
        .unwrap();
    config.set("mime.command.timeout", "2s").unwrap();

    let list_policy = ReadPolicy::builder()
        .conversion_policy(
            ConversionPolicy::env_friendly()
                .into_builder()
                .collection_policy(
                    CollectionConversionPolicy::env_friendly()
                        .into_builder()
                        .delimiters([',', ';'])
                        .build(),
                )
                .build(),
        )
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let mapping_policy = ReadPolicy::builder()
        .conversion_policy(
            ConversionPolicy::env_friendly()
                .into_builder()
                .collection_policy(
                    CollectionConversionPolicy::env_friendly()
                        .into_builder()
                        .delimiters([';'])
                        .build(),
                )
                .build(),
        )
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let duration_policy = ReadPolicy::builder()
        .conversion_policy(
            ConversionPolicy::env_friendly()
                .into_builder()
                .duration_policy(DurationConversionPolicy::default())
                .build(),
        )
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();

    let list_config = config.read_with(&list_policy);
    assert_eq!(
        list_config
            .get_any_interpolated_or::<Vec<String>>(["mime.detector.fallbacks"], Vec::<String>::new())
            .unwrap(),
        ["file", "repository"]
    );

    let mapping_config = config.read_with(&mapping_policy);
    assert_eq!(
        mapping_config
            .get_any_interpolated_or::<Vec<String>>(["mime.ambiguous.mime.mapping"], Vec::<String>::new(),)
            .unwrap(),
        ["webm:video/webm,audio/webm", "ogg:video/ogg,audio/ogg"]
    );

    let duration_config = config.read_with(&duration_policy);
    assert_eq!(
        duration_config
            .get_any_interpolated_or::<Duration>(["mime.command.timeout"], Duration::from_secs(30))
            .unwrap(),
        Duration::from_secs(2)
    );
}

#[test]
fn read_with_allows_explicit_environment_interpolation_for_downstream_readers() {
    let _guard = env_test_lock();
    let env_name = format!("QUBIT_RS_CONFIG_DOWNSTREAM_ENV_{}", std::process::id());
    let _env_var = EnvVarGuard::set(&env_name, "Bearer environment-token");

    let mut config = Config::new();
    config
        .set("http.default_headers.authorization", format!("${{{env_name}}}"))
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
}

#[test]
fn mime_style_env_config_options_load_prefixed_keys_without_normalization() {
    let _guard = env_test_lock();
    let suffix = std::process::id();
    let matching = format!("QUBIT_RS_CONFIG_DOWNSTREAM_OPTION_{suffix}");
    let unrelated = format!("RS_CONFIG_DOWNSTREAM_UNRELATED_{suffix}");
    let _matching_var = EnvVarGuard::set(&matching, "repository");
    let _unrelated_var = EnvVarGuard::set(&unrelated, "must-not-load");

    let options = EnvConfigOptions::builder()
        .prefix("QUBIT_RS_CONFIG_DOWNSTREAM_")
        .build();
    let config = Config::from_env_options(options).unwrap();

    assert_eq!(config.get::<String>(&matching).unwrap(), "repository");
    assert!(!config.contains(&unrelated).unwrap());
}

#[test]
fn rs_http_config_error_mappings_receive_stable_source_kinds() {
    let mut config = Config::new();
    config.set("http.retry.max_attempts", "three").unwrap();
    config.set("http.max_redirects", "not-a-number").unwrap();
    config.set_null("http.base_url", DataType::String).unwrap();

    let type_mismatch = config
        .get_strict::<u64>("http.retry.max_attempts")
        .expect_err("strict HTTP option reads should preserve type mismatch");
    assert_eq!(type_mismatch.kind(), ConfigErrorKind::TypeMismatch);
    assert_eq!(type_mismatch.path(), Some("http.retry.max_attempts"));

    let conversion = config
        .get::<u64>("http.max_redirects")
        .expect_err("invalid HTTP option values should preserve conversion errors");
    assert_eq!(conversion.kind(), ConfigErrorKind::Conversion);
    assert_eq!(conversion.path(), Some("http.max_redirects"));

    let no_value = config
        .get::<String>("http.base_url")
        .expect_err("unset HTTP options should preserve missing-value errors");
    assert_eq!(no_value.kind(), ConfigErrorKind::Value);
    assert!(no_value.value_missing().unwrap().is_unset());
    assert_eq!(no_value.path(), Some("http.base_url"));
}
