/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # `CompositeConfigSource` tests

use qubit_config::{
    Config,
    source::{
        CompositeConfigSource, ConfigSource, EnvConfigSource, PropertiesConfigSource,
        TomlConfigSource,
    },
};

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

// ============================================================================
// CompositeConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_composite_config_source {
    #[allow(unused_imports)]
    use super::{
        CompositeConfigSource, Config, ConfigSource, EnvConfigSource, PathBuf,
        PropertiesConfigSource, TomlConfigSource, fixture,
    };

    #[test]
    fn test_new_composite_is_empty() {
        let composite = CompositeConfigSource::new();
        assert!(composite.is_empty());
        assert_eq!(composite.len(), 0);
    }

    #[test]
    fn test_add_source_increases_len() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        assert_eq!(composite.len(), 1);
        assert!(!composite.is_empty());
    }

    #[test]
    fn test_add_multiple_sources() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(PropertiesConfigSource::from_file(fixture(
            "basic.properties",
        )));
        assert_eq!(composite.len(), 2);
    }

    #[test]
    fn test_load_merges_sources_in_order() {
        // basic.toml sets host=localhost, override.toml sets host=production-server
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        composite.load(&mut config).unwrap();

        // Later source wins
        assert_eq!(config.get_string("host").unwrap(), "production-server");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("port").unwrap(), 443);
        // Keys only in first source are still present
        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
    }

    #[test]
    fn test_load_empty_composite_does_nothing() {
        let composite = CompositeConfigSource::new();
        let mut config = Config::new();
        composite.load(&mut config).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn test_load_stops_on_first_error() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file("/nonexistent/path.toml"));
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));

        let mut config = Config::new();
        let result = composite.load(&mut config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_creates_empty_composite() {
        let composite = CompositeConfigSource::default();
        assert!(composite.is_empty());
    }

    #[test]
    fn test_composite_with_env_override() {
        unsafe {
            std::env::set_var("CTEST_HOST", "env-host");
        }

        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(EnvConfigSource::with_prefix("CTEST_"));

        let mut config = Config::new();
        composite.load(&mut config).unwrap();

        // env source overrides toml
        assert_eq!(config.get_string("host").unwrap(), "env-host");
        // toml-only keys still present
        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");

        unsafe {
            std::env::remove_var("CTEST_HOST");
        }
    }

    #[test]
    fn test_merge_from_composite_config_source() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        config.merge_from_source(&composite).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "production-server");
    }

    #[test]
    fn test_add_returns_mutable_ref_for_chaining() {
        // Verify the builder-style chaining works
        let mut composite = CompositeConfigSource::new();
        composite
            .add(TomlConfigSource::from_file(fixture("basic.toml")))
            .add(TomlConfigSource::from_file(fixture("override.toml")));

        assert_eq!(composite.len(), 2);
    }
}
