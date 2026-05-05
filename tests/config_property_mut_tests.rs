/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for guarded mutable property access.

use qubit_config::{
    Config,
    ConfigError,
};

#[test]
fn test_config_property_mut_updates_non_final_property() {
    let mut config = Config::new();
    config
        .set("server.port", 8080i32)
        .expect("setting config value should succeed");

    {
        let mut property = config
            .get_property_mut("server.port")
            .expect("mutable access should succeed")
            .expect("property should exist");
        property
            .set_description(Some("HTTP port".to_string()))
            .expect("description update should succeed");
        property.set(9090i32).expect("value update should succeed");
    }

    let property = config
        .get_property("server.port")
        .expect("property should remain present");
    assert_eq!(property.description(), Some("HTTP port"));
    assert_eq!(config.get::<i32>("server.port").unwrap(), 9090);
}

#[test]
fn test_config_property_mut_rejects_mutation_after_marking_final() {
    let mut config = Config::new();
    config
        .set("server.host", "localhost")
        .expect("setting config value should succeed");

    {
        let mut property = config
            .get_property_mut("server.host")
            .expect("mutable access should succeed")
            .expect("property should exist");
        property
            .set_final(true)
            .expect("marking property final should succeed");

        assert!(matches!(
            property.set("example.com"),
            Err(ConfigError::PropertyIsFinal(name)) if name == "server.host"
        ));
        assert!(matches!(
            property.clear(),
            Err(ConfigError::PropertyIsFinal(name)) if name == "server.host"
        ));
    }
    assert_eq!(config.get_string("server.host").unwrap(), "localhost");
}
