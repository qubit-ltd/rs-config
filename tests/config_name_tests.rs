/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for configuration key argument adapters.

use qubit_config::{
    Config,
    ConfigReader,
};

#[test]
fn test_config_name_accepts_str_string_and_string_ref() {
    let mut config = Config::new();
    config
        .set("server.host", "localhost")
        .expect("setting config value should succeed");

    let owned = String::from("server.host");
    let borrowed = String::from("server.host");

    assert_eq!(config.get_string("server.host").unwrap(), "localhost");
    assert_eq!(config.get_string(owned).unwrap(), "localhost");
    assert_eq!(config.get_string(&borrowed).unwrap(), "localhost");
}

#[test]
fn test_config_name_resolves_relative_to_prefix_view() {
    let mut config = Config::new();
    config
        .set("http.host", "localhost")
        .expect("setting config value should succeed");

    let view = config.prefix_view("http");
    let name = String::from("host");

    assert!(ConfigReader::contains(&view, &name));
    assert_eq!(view.get_string(name).unwrap(), "localhost");
}
