/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for configuration key list argument adapters.

use qubit_config::Config;

#[test]
fn test_config_names_accepts_str_slice_array_and_vec() {
    let mut config = Config::new();
    config
        .set("legacy.port", 8080i32)
        .expect("setting config value should succeed");

    let slice: &[&str] = &["server.port", "legacy.port"];
    let vec_names = vec!["server.port", "legacy.port"];

    assert_eq!(config.get_any::<i32>(slice).unwrap(), 8080);
    assert_eq!(
        config
            .get_any::<i32>(["server.port", "legacy.port"])
            .unwrap(),
        8080
    );
    assert_eq!(
        config
            .get_any::<i32>(&["server.port", "legacy.port"])
            .unwrap(),
        8080
    );
    assert_eq!(config.get_any::<i32>(vec_names).unwrap(), 8080);
}

#[test]
fn test_config_names_accepts_owned_string_lists() {
    let mut config = Config::new();
    config
        .set("APP_HOST", "localhost")
        .expect("setting config value should succeed");

    let array = [String::from("server.host"), String::from("APP_HOST")];
    let vec_names = vec![String::from("server.host"), String::from("APP_HOST")];

    assert_eq!(config.get_string_any(&array).unwrap(), "localhost");
    assert_eq!(config.get_string_any(array).unwrap(), "localhost");
    assert_eq!(config.get_string_any(&vec_names).unwrap(), "localhost");
    assert_eq!(config.get_string_any(vec_names).unwrap(), "localhost");
}
