// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for the `ConfigSource` trait contract.

use qubit_config::Config;
use qubit_config::ConfigResult;
use qubit_config::options::ReadPolicy;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadSession;

struct InlineSource {
    key: &'static str,
    value: &'static str,
}

impl ConfigSource for InlineSource {
    fn source_id(&self) -> String {
        "inline".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_with_session(&self, session: &mut SourceLoadSession<'_>) -> ConfigResult<Config> {
        session.consume_nodes(1)?;
        session.consume_properties(1)?;
        let mut config = Config::new();
        config.set(self.key, self.value)?;
        Ok(config)
    }
}

struct MetadataSource;

impl ConfigSource for MetadataSource {
    fn source_id(&self) -> String {
        "metadata".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_with_session(&self, session: &mut SourceLoadSession<'_>) -> ConfigResult<Config> {
        session.consume_nodes(1)?;
        session.consume_properties(1)?;
        let mut config = Config::new();
        config.set_description(Some("source".to_string()));
        config.set_default_read_policy(ReadPolicy::env_friendly());
        config.set("server.host", "localhost")?;
        Ok(config)
    }
}

#[test]
fn test_config_source_load_populates_config() {
    let source = InlineSource {
        key: "server.host",
        value: "localhost",
    };
    let mut config = Config::new();

    config
        .merge_properties_from_source(&source)
        .expect("inline source should load successfully");

    assert_eq!(config.get::<String>("server.host").unwrap(), "localhost");
}

#[test]
fn test_config_merge_properties_from_source_uses_trait_implementation() {
    let source = InlineSource {
        key: "server.port",
        value: "8080",
    };
    let mut config = Config::new();

    config
        .merge_properties_from_source(&source)
        .expect("trait source should merge successfully");

    assert_eq!(config.get::<u16>("server.port").unwrap(), 8080);
}

#[test]
fn test_config_source_default_load_and_merge() {
    let source = InlineSource {
        key: "server.name",
        value: "api",
    };
    let mut config = Config::new();
    config
        .merge_properties_from_source(&source)
        .expect("inline source should load successfully");

    assert_eq!(config.get::<String>("server.name").unwrap(), "api");
}

#[test]
fn test_property_merge_ignores_source_metadata() {
    let source = MetadataSource;
    let loaded = Config::from_source(&source).expect("source should load");
    assert_eq!(loaded.description(), Some("source"));
    assert_eq!(loaded.default_read_policy(), &ReadPolicy::env_friendly());

    let target_policy = ReadPolicy::default();
    let mut target = Config::new();
    target.set_description(Some("target".to_string()));
    target.set_default_read_policy(target_policy.clone());
    target
        .merge_properties_from_source(&source)
        .expect("source properties should merge");

    assert_eq!(target.description(), Some("target"));
    assert_eq!(target.default_read_policy(), &target_policy);
    assert_eq!(target.get::<String>("server.host").unwrap(), "localhost");
}
