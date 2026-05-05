/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for typed `FromConfig` parsing behavior.

use std::time::Duration;

use qubit_config::{
    Config,
    ConfigError,
};

#[test]
fn test_from_config_converts_scalar_string_to_duration() {
    let mut config = Config::new();
    config
        .set("server.timeout_secs", "30")
        .expect("setting config value should succeed");

    let timeout = config
        .get::<Duration>("server.timeout_secs")
        .expect("duration should parse from scalar string");

    assert_eq!(timeout, Duration::from_millis(30));
}

#[test]
fn test_from_config_reports_keyed_conversion_error() {
    let mut config = Config::new();
    config
        .set("server.port", "not-a-port")
        .expect("setting config value should succeed");

    let error = config
        .get::<u16>("server.port")
        .expect_err("invalid integer should fail conversion");

    assert!(matches!(
        error,
        ConfigError::ConversionError { key, .. } if key == "server.port"
    ));
}
