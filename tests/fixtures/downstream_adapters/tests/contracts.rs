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
use qubit_config::ConfigErrorKind;
use qubit_http::HttpClientOptions;
use qubit_http::HttpConfigErrorKind;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct IocServerProperties {
    host: String,
    port: u16,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosedProperties {
    host: String,
    port: u16,
}

#[test]
fn test_http_adapter_reads_config_and_rejects_invalid_and_unknown_values() {
    let mut config = Config::new();
    config.set("user_agent", "contract-agent").unwrap();
    config.set("max_redirects", 4_u64).unwrap();
    let options = HttpClientOptions::from_config(&config).unwrap();
    assert_eq!(options.user_agent.as_deref(), Some("contract-agent"));
    assert_eq!(options.max_redirects, Some(4));

    let mut invalid = Config::new();
    invalid.set("max_redirects", "not-a-number").unwrap();
    let error = HttpClientOptions::from_config(&invalid).unwrap_err();
    assert_eq!(error.kind, HttpConfigErrorKind::TypeError);
    assert_eq!(error.path, "max_redirects");

    let mut unknown = Config::new();
    unknown.set("unrecognized", true).unwrap();
    let error = HttpClientOptions::from_config(&unknown).unwrap_err();
    assert_eq!(error.kind, HttpConfigErrorKind::ConfigError);
    let source = Error::source(&error).expect("unknown keys retain the config error");
    let config_error = source.downcast_ref::<ConfigError>().expect("real ConfigError source");
    assert_eq!(config_error.kind(), ConfigErrorKind::UnknownProperty);
}

#[test]
fn test_ioc_adapter_deserializes_defaults_and_preserves_config_error_source() {
    let mut config = Config::new();
    config.set("server.host", "127.0.0.1").unwrap();
    config.set("server.port", 8080_u64).unwrap();
    let properties: IocServerProperties = qubit_ioc::config::deserialize_properties(&config, "server").unwrap();
    assert_eq!(properties.host, "127.0.0.1");
    assert_eq!(properties.port, 8080);
    assert!(properties.enabled);

    let mut invalid = Config::new();
    invalid.set("server.host", "127.0.0.1").unwrap();
    invalid.set("server.port", "not-a-port").unwrap();
    let error = qubit_ioc::config::deserialize_properties::<IocServerProperties>(&invalid, "server").unwrap_err();
    let source = Error::source(&error).expect("FactoryError wraps the original ConfigError");
    let config_error = source.downcast_ref::<ConfigError>().expect("real ConfigError source");
    assert_eq!(config_error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(config_error.path(), Some("server.port"));

    let mut closed = Config::new();
    closed.set("server.host", "127.0.0.1").unwrap();
    closed.set("server.port", 8080_u64).unwrap();
    let properties: ClosedProperties = qubit_ioc::config::deserialize_properties(&closed, "server").unwrap();
    assert_eq!(properties.host, "127.0.0.1");
    assert_eq!(properties.port, 8080);

    closed.set("server.extra", true).unwrap();
    let error = qubit_ioc::config::deserialize_properties::<ClosedProperties>(&closed, "server").unwrap_err();
    let source = Error::source(&error).expect("unknown-field failure keeps its config source");
    let config_error = source.downcast_ref::<ConfigError>().expect("real ConfigError source");
    assert_eq!(config_error.kind(), ConfigErrorKind::Deserialize);
}
