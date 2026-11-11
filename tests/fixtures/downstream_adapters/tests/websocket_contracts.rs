// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_websocket::WsClientBuilder;

fn error_from(config: &Config) -> ConfigError {
    match WsClientBuilder::from_config(config, "ws") {
        Ok(_) => panic!("configuration should be rejected"),
        Err(error) => error,
    }
}

fn valid_config() -> Config {
    let mut config = Config::new();
    config.set("ws.connect_timeout_ms", 2300_u64).expect("set timeout");
    config
        .set("ws.subprotocols", vec!["jobs.v1", "jobs.v2"])
        .expect("set subprotocols");
    config
}

#[test]
fn test_nonempty_section_builds() {
    let client = WsClientBuilder::from_config(&valid_config(), "ws")
        .expect("read valid WebSocket section")
        .build();
    assert!(client.is_ok());
}

#[test]
fn test_same_name_prefix_is_isolated() {
    let mut config = valid_config();
    config.set("ws2.unknown", true).expect("set neighboring key");
    assert!(
        WsClientBuilder::from_config(&config, "ws")
            .expect("read ws only")
            .build()
            .is_ok()
    );
}

#[test]
fn test_exact_section_presence_is_required() {
    let mut scalar = Config::new();
    scalar.set("ws", "disabled").expect("set scalar");
    let error = error_from(&scalar);
    assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(error.path(), Some("ws"));

    let mut neighbor = Config::new();
    neighbor
        .set("ws2.connect_timeout_ms", 2300_u64)
        .expect("set neighboring section");
    let error = error_from(&neighbor);
    assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(error.path(), Some("ws"));
}

#[test]
fn test_unknown_property_reports_full_path() {
    let mut config = valid_config();
    config.set("ws.typo", true).expect("set unknown key");
    let error = error_from(&config);
    assert_eq!(error.kind(), ConfigErrorKind::UnknownProperty);
    assert_eq!(error.unknown_property_paths(), Some(["ws.typo".to_owned()].as_slice()));
}

#[test]
fn test_invalid_subprotocol_is_checked_by_build() {
    let mut config = Config::new();
    config.set("ws.connect_timeout_ms", 2300_u64).expect("set timeout");
    config
        .set("ws.subprotocols", vec!["jobs.v1", "bad token"])
        .expect("set invalid subprotocol list");
    let builder = WsClientBuilder::from_config(&config, "ws").expect("map list to builder");
    assert!(builder.build().is_err());
}

#[test]
fn test_conversion_error_reports_full_path() {
    let mut config = Config::new();
    config
        .set("ws.connect_timeout_ms", "invalid")
        .expect("set invalid timeout");
    let error = error_from(&config);
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("ws.connect_timeout_ms"));
}

#[test]
fn test_usize_overflow_reports_full_path() {
    let mut config = Config::new();
    config
        .set("ws.limits.max_inbound_frame_bytes", u128::MAX)
        .expect("set oversized limit");
    let error = error_from(&config);
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("ws.limits.max_inbound_frame_bytes"));
}
