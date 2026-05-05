/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the `ConfigSource` trait contract.

use qubit_config::{
    Config,
    ConfigResult,
    source::ConfigSource,
};

struct InlineSource {
    key: &'static str,
    value: &'static str,
}

impl ConfigSource for InlineSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        config.set(self.key, self.value)
    }
}

#[test]
fn test_config_source_load_populates_config() {
    let source = InlineSource {
        key: "server.host",
        value: "localhost",
    };
    let mut config = Config::new();

    source
        .load(&mut config)
        .expect("inline source should load successfully");

    assert_eq!(config.get_string("server.host").unwrap(), "localhost");
}

#[test]
fn test_config_merge_from_source_uses_trait_implementation() {
    let source = InlineSource {
        key: "server.port",
        value: "8080",
    };
    let mut config = Config::new();

    config
        .merge_from_source(&source)
        .expect("trait source should merge successfully");

    assert_eq!(config.get::<u16>("server.port").unwrap(), 8080);
}
