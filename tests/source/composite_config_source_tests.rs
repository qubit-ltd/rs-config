// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg(feature = "toml")]

// # `CompositeConfigSource` tests

use std::path::PathBuf;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;
use qubit_config::source::TomlConfigSource;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn load_source(
    config: &mut Config,
    source: &dyn ConfigSource,
) -> ConfigResult<()> {
    config.merge_properties_from_source(source)
}

// ============================================================================
// CompositeConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_composite_config_source {
    use qubit_budget::BudgetError;

    use super::CompositeConfigSource;
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::EnvConfigSource;
    use super::PropertiesConfigSource;
    use super::SourceLimitKind;
    use super::SourceLimits;
    use super::TomlConfigSource;
    use super::fixture;
    use super::load_source;

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
        // basic.toml sets host=localhost, override.toml sets
        // host=production-server
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        load_source(&mut config, &composite).unwrap();

        // Later source wins
        assert_eq!(config.get::<String>("host").unwrap(), "production-server");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("port").unwrap(), 443);
        // Keys only in first source are still present
        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
    }

    #[test]
    fn test_load_empty_composite_does_nothing() {
        let composite = CompositeConfigSource::new();
        let mut config = Config::new();
        load_source(&mut config, &composite).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn test_load_stops_on_first_error() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file("/nonexistent/path.toml"));
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));

        let result = composite.load();
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
        load_source(&mut config, &composite).unwrap();

        // env source overrides toml
        assert_eq!(config.get::<String>("host").unwrap(), "env-host");
        // toml-only keys still present
        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");

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
        config.merge_properties_from_source(&composite).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "production-server");
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

    #[test]
    fn test_composite_load_is_transactional_when_later_source_fails() {
        let dir = tempfile::tempdir().unwrap();
        let defaults = dir.path().join("defaults.toml");
        let overrides = dir.path().join("overrides.toml");
        std::fs::write(&defaults, "new_key = \"new\"\n").unwrap();
        std::fs::write(&overrides, "locked = \"attempted\"\n").unwrap();

        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(&defaults));
        composite.add(TomlConfigSource::from_file(&overrides));

        let mut config = Config::new();
        config.set("locked", "old").unwrap();
        config.set_final("locked", true).unwrap();

        let result = config.merge_properties_from_source(&composite);

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get::<String>("locked").unwrap(), "old");
        assert!(!config.contains("new_key").unwrap());
    }

    #[test]
    fn test_composite_enforces_aggregate_property_budget() {
        let mut composite = CompositeConfigSource::new()
            .with_limits(SourceLimits::default().with_max_properties(1));
        composite.add(PropertiesConfigSource::from_content("first=1\n"));
        composite.add(PropertiesConfigSource::from_content("second=2\n"));

        let error = composite.load().expect_err(
            "the second property should exceed the aggregate budget",
        );

        assert_eq!(
            error.source_budget_id(),
            Some("composite configuration source")
        );
        assert!(matches!(
            error.budget_error(),
            Some(BudgetError::Insufficient {
                resource: SourceLimitKind::PropertyCount,
                limit: 1,
                remaining: 0,
                requested: 1,
            })
        ));
    }

    #[test]
    fn test_composite_rejects_source_before_loading_it() {
        let mut composite = CompositeConfigSource::new()
            .with_limits(SourceLimits::default().with_max_sources(1));
        composite.add(PropertiesConfigSource::from_content("first=1\n"));
        composite.add(TomlConfigSource::from_file(
            "/path-that-must-not-be-opened.toml",
        ));

        let error = composite
            .load()
            .expect_err("the aggregate source count should fail first");

        assert!(matches!(
            error.budget_error(),
            Some(BudgetError::Insufficient {
                resource: SourceLimitKind::SourceCount,
                limit: 1,
                remaining: 0,
                requested: 1,
            })
        ));
    }
}
