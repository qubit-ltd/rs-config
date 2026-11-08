// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::error::Error;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_http::HttpClientOptions;
use qubit_http::HttpConfigError;
use qubit_http::HttpConfigErrorKind;
use qubit_ioc::config::deserialize_properties;
use qubit_mime::MimeConfig;
use qubit_spi::ProviderSelectionTargetRef;
use qubit_web::ConfiguredWeb;
use qubit_web::config::stream_admission_limits_from_config;
use qubit_websocket::WsClientBuilder;
use serde::Deserialize;

const MIME_PRIORITY_CHILD: &str = "QUBIT_DOWNSTREAM_MIME_PRIORITY_CHILD";
const MIME_PRIORITY_COMPLETED: &str = "QUBIT_MIME_PRIORITY_ASSERTIONS_COMPLETED";

fn run_mime_priority_child(test_name: &str) -> std::process::Output {
    std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env_clear()
        .env(MIME_PRIORITY_CHILD, "1")
        .env("QUBIT_MIME_DETECTOR_DEFAULT", "file")
        .output()
        .expect("isolated priority test process should start")
}

fn mime_priority_child_completed(output: &std::process::Output) -> bool {
    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == MIME_PRIORITY_COMPLETED)
}

#[test]
fn test_http_maps_config_errors_with_kind_path_and_source() {
    let missing = HttpConfigError::from(ConfigError::PropertyNotFound("http.base_url".into()));
    assert_eq!(missing.kind, HttpConfigErrorKind::ConfigError);
    assert_eq!(missing.path, "http.base_url");
    assert!(Error::source(&missing).is_some());

    let mut config = Config::new();
    config.set("http.json.max_nodes", "not-a-number").unwrap();
    let section = config.section("http").unwrap();
    let error = HttpClientOptions::from_config(&section).expect_err("invalid numeric conversion");
    assert_eq!(error.kind, HttpConfigErrorKind::TypeError);
    assert_eq!(error.path, "http.json.max_nodes");
    assert!(Error::source(&error).is_some());

    let type_error = config.get_strict::<bool>("http.json.max_nodes").unwrap_err();
    let mapped = HttpConfigError::from(type_error);
    assert_eq!(mapped.kind, HttpConfigErrorKind::TypeError);
    assert_eq!(mapped.path, "http.json.max_nodes");
    assert!(Error::source(&mapped).is_some());
}

#[test]
fn test_mime_reads_values_lists_mappings_durations_and_interpolation() {
    let mut config = Config::new();
    config.set("mime.detector.default", "${chosen}").unwrap();
    config.set("chosen", "repository").unwrap();
    config.set("mime.detector.fallbacks", vec!["file", "local"]).unwrap();
    config.set("mime.command.timeout", "2s").unwrap();
    config
        .set("mime.ambiguous.mime.mapping", "webm:video/x-webm,audio/x-webm")
        .unwrap();
    let loaded = MimeConfig::from_config(&config).expect("MIME adapter should read config");
    match loaded.mime_detector_selection().target() {
        ProviderSelectionTargetRef::Chain { selectors, .. } => {
            let selected: Vec<_> = selectors.iter().map(|selector| selector.as_str()).collect();
            assert_eq!(selected, ["repository", "file", "local"]);
        }
        ProviderSelectionTargetRef::Named(selector) => {
            panic!("configured fallbacks should produce a chain, got {selector}")
        }
        ProviderSelectionTargetRef::Auto => {
            panic!("explicit MIME config should select named providers")
        }
        other => panic!("unexpected provider selection: {other:?}"),
    }
    assert_eq!(loaded.command_timeout(), std::time::Duration::from_secs(2));
    assert_eq!(
        loaded.ambiguous_mime_mapping().get("webm"),
        Some(&["video/x-webm".into(), "audio/x-webm".into()])
    );

    let mut bad = Config::new();
    bad.set("mime.detector.default", "${unset}").unwrap();
    assert!(
        MimeConfig::from_config(&bad).is_err(),
        "explicit interpolation should be evaluated"
    );
}

#[test]
fn test_mime_config_key_wins_over_environment_fallback() {
    if std::env::var_os(MIME_PRIORITY_CHILD).is_some() {
        let mut config = Config::new();
        config.set("mime.detector.default", "repository").unwrap();
        let loaded = MimeConfig::from_config(&config).expect("MIME config key should be read");
        match loaded.mime_detector_selection().target() {
            ProviderSelectionTargetRef::Named(selector) => {
                assert_eq!(selector.as_str(), "repository")
            }
            ProviderSelectionTargetRef::Chain { selectors, .. } => {
                assert_eq!(selectors.first().map(|selector| selector.as_str()), Some("repository"));
            }
            ProviderSelectionTargetRef::Auto => {
                panic!("explicit MIME config should select a provider")
            }
            other => panic!("unexpected provider selection: {other:?}"),
        }
        println!("{MIME_PRIORITY_COMPLETED}");
        return;
    }

    let output = run_mime_priority_child("test_mime_config_key_wins_over_environment_fallback");
    assert!(
        mime_priority_child_completed(&output),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_mime_priority_child_rejects_zero_tests() {
    let output = run_mime_priority_child("__qubit_config_missing_priority_child__");
    assert!(output.status.success(), "the zero-test control must exit successfully");
    assert!(!mime_priority_child_completed(&output));
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct IocSettings {
    endpoint: String,
    #[serde(default = "default_retries")]
    retries: u8,
}
fn default_retries() -> u8 {
    3
}

#[test]
fn test_ioc_deserializes_properties_and_preserves_config_error() {
    let mut config = Config::new();
    config.set("service.endpoint", "https://example.test").unwrap();
    let value: IocSettings = deserialize_properties(&config, "service").unwrap();
    assert_eq!(
        value,
        IocSettings {
            endpoint: "https://example.test".into(),
            retries: 3
        }
    );

    config.set("service.extra", true).unwrap();
    let error = deserialize_properties::<IocSettings>(&config, "service").unwrap_err();
    let source = Error::source(&error).expect("factory error should preserve unknown-field ConfigError");
    assert!(source.downcast_ref::<ConfigError>().is_some());

    let mut bad = Config::new();
    bad.set("service.endpoint", vec!["one", "two"]).unwrap();
    let error = deserialize_properties::<IocSettings>(&bad, "service").unwrap_err();
    let source = Error::source(&error).expect("factory error should preserve ConfigError");
    assert!(source.downcast_ref::<ConfigError>().is_some());
}

#[test]
fn test_websocket_reads_bounded_prefix_and_reports_unknown_or_bad_values() {
    let mut config = Config::new();
    config.set("websocket.connect_timeout_ms", 250u64).unwrap();
    config.set("websocket.limits.max_inbound_frame_bytes", 4096u64).unwrap();
    config.set("other.connect_timeout_ms", "bad").unwrap();
    let _builder = WsClientBuilder::from_config(&config, "websocket").unwrap();

    config.set("websocket.unknown", true).unwrap();
    assert!(WsClientBuilder::from_config(&config, "websocket").is_err());

    let mut bad = Config::new();
    bad.set("websocket.connect_timeout_ms", "not-a-number").unwrap();
    assert!(WsClientBuilder::from_config(&bad, "websocket").is_err());
}

#[test]
fn test_web_config_adapter_contracts() {
    let mut config = Config::new();
    config.set("address", "127.0.0.1:0").unwrap();
    let (options, _) = ConfiguredWeb::from_config(&config).unwrap().into_parts();
    assert_eq!(options.max_transport_connections(), 1024);
    assert_eq!(options.http2_max_concurrent_streams(), 200);

    config.set("transport.max_connections", 3_u64).unwrap();
    config.set("http.max_body_bytes", 4096_u64).unwrap();
    config.set("transport.http1_max_buffer_bytes", 8192_u64).unwrap();
    config
        .set("transport.http2_max_header_list_bytes", u64::from(u32::MAX))
        .unwrap();
    let (options, limits) = ConfiguredWeb::from_config(&config).unwrap().into_parts();
    assert_eq!(
        options.address(),
        "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap()
    );
    assert_eq!(options.max_transport_connections(), 3);
    assert_eq!(options.http1_max_buffer_bytes(), Some(8192));
    assert_eq!(options.http2_max_header_list_bytes(), Some(u32::MAX));
    assert_eq!(limits.max_body_bytes(), 4096);
}

#[test]
fn test_web_config_adapter_rejects_invalid_fields() {
    assert_eq!(
        ConfiguredWeb::from_config(&Config::new()).unwrap_err().field(),
        "address"
    );
    let mut wrong_address = Config::new();
    wrong_address.set("address", 8080_u16).unwrap();
    assert_eq!(
        ConfiguredWeb::from_config(&wrong_address).unwrap_err().field(),
        "address"
    );
    let mut wrong_type = Config::new();
    wrong_type.set("address", "127.0.0.1:0").unwrap();
    wrong_type.set("transport.max_connections", "not-a-number").unwrap();
    assert_eq!(
        ConfiguredWeb::from_config(&wrong_type).unwrap_err().field(),
        "transport.max_connections"
    );

    for (field, value) in [
        ("transport.http1_max_buffer_bytes", 8191_u64),
        ("transport.http2_max_header_list_bytes", 0_u64),
        ("transport.http2_max_header_list_bytes", u64::from(u32::MAX) + 1),
    ] {
        let mut config = Config::new();
        config.set("address", "127.0.0.1:0").unwrap();
        config.set(field, value).unwrap();
        assert_eq!(ConfiguredWeb::from_config(&config).unwrap_err().field(), field);
    }
}

#[test]
fn test_web_stream_admission_config_adapter_is_independent_of_listener_config() {
    let mut config = Config::new();
    config
        .set("stream_admission.max_body_bytes", 4096_u64)
        .unwrap();
    config
        .set("stream_admission.max_concurrent_admissions", 3_u64)
        .unwrap();
    config
        .set("stream_admission.admission_timeout_ms", 1500_u64)
        .unwrap();

    let limits = stream_admission_limits_from_config(&config).unwrap();
    assert_eq!(limits.max_body_bytes(), 4096);
    assert_eq!(limits.max_concurrent_admissions(), 3);
    assert_eq!(
        limits.admission_timeout(),
        std::time::Duration::from_millis(1500)
    );
}

#[test]
fn test_web_stream_admission_config_adapter_reports_invalid_key() {
    let mut config = Config::new();
    config
        .set("stream_admission.max_concurrent_admissions", "not-a-number")
        .unwrap();

    assert_eq!(
        stream_admission_limits_from_config(&config)
            .unwrap_err()
            .field(),
        "stream_admission.max_concurrent_admissions"
    );
}
