// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for the `ConfigSource` trait contract.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::options::ReadPolicy;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;
use qubit_datatype::DataType;

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

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set(self.key, self.value)
    }
}

struct MetadataSource;

struct OverrideLoadSource {
    limits: SourceLimits,
}

impl ConfigSource for OverrideLoadSource {
    fn source_id(&self) -> String {
        "override:<memory>".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        self.limits
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("value", 8080_u16)
    }

    fn load(&self) -> ConfigResult<Config> {
        let mut config = Config::new();
        config.set("value", 999_u16)?;
        Ok(config)
    }
}

struct FailingLoadSource;

impl ConfigSource for FailingLoadSource {
    fn source_id(&self) -> String {
        "failing:<memory>".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("temporary", 1_u16)?;
        Err(ConfigError::PropertyNotFound("expected-source-failure".to_owned()))
    }

    fn load(&self) -> ConfigResult<Config> {
        Ok(Config::new())
    }
}

impl ConfigSource for MetadataSource {
    fn source_id(&self) -> String {
        "metadata".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set_description(Some("source".to_string()));
        context.set_default_read_policy(ReadPolicy::env_friendly());
        context.set("server.host", "localhost")
    }
}

struct FinalSource {
    limits: SourceLimits,
}

impl ConfigSource for FinalSource {
    fn source_id(&self) -> String {
        "final:<memory>".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        self.limits
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("locked", 1_u16)?;
        context.set_final("locked", true)?;
        context.set_final("locked", true)?;
        if self.limits.max_properties() <= 1 {
            let error = context
                .set("second", 2_u16)
                .expect_err("the second assignment must still exceed the spent budget");
            assert!(matches!(error, ConfigError::SourceLimitExceeded { .. }));
            return Ok(());
        }
        let error = context
            .set_final("locked", false)
            .expect_err("a final property cannot be unlocked");
        assert!(matches!(error, ConfigError::PropertyIsFinal(ref key) if key == "locked"));
        let error = context
            .set("locked", 2_u16)
            .expect_err("a final property cannot be overwritten");
        assert!(matches!(error, ConfigError::PropertyIsFinal(ref key) if key == "locked"));
        let error = context
            .set_null("locked", DataType::String)
            .expect_err("a final property cannot be replaced with null");
        assert!(matches!(error, ConfigError::PropertyIsFinal(ref key) if key == "locked"));
        context.set("unlocked", 3_u16)?;
        context.set_final("unlocked", false)?;
        context.set_final("unlocked", false)?;

        let error = context
            .set_final("missing", true)
            .expect_err("missing properties cannot be marked final");
        assert!(matches!(error, ConfigError::PropertyNotFound(ref key) if key == "missing"));
        let error = context
            .set_final("invalid..key", true)
            .expect_err("invalid property names must be rejected");
        assert!(matches!(&error, ConfigError::SourceInvalidKey { key, source_id, .. }
            if key == "invalid..key" && source_id == "final:<memory>"));
        assert_eq!(error.path(), Some("invalid..key"));
        Ok(())
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
fn test_source_merge_final_error_has_inline_context_and_is_atomic() {
    let source = InlineSource {
        key: "locked",
        value: "new",
    };
    let mut target = Config::builder().description("target description").build();
    target.set_default_read_policy(ReadPolicy::env_friendly());
    target.set("locked", "old").unwrap();
    target.set_final("locked", true).unwrap();
    target.set("kept", "value").unwrap();
    let before = target.clone();

    let error = target.merge_properties_from_source(&source).unwrap_err();

    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::PropertyIsFinal);
    assert_eq!(error.path(), Some("locked"));
    assert_eq!(error.source_id(), Some("inline"));
    assert_eq!(target.get::<String>("locked").unwrap(), "old");
    assert_eq!(target.description(), Some("target description"));
    assert_eq!(target.default_read_policy(), &ReadPolicy::env_friendly());
    assert_eq!(target.get::<String>("kept").unwrap(), "value");
    assert_eq!(target, before);
}

#[test]
fn test_managed_source_entry_points_use_context_executor() {
    let source = OverrideLoadSource {
        limits: SourceLimits::default(),
    };
    assert_eq!(source.load().unwrap().get::<u16>("value").unwrap(), 999);
    assert_eq!(Config::from_source(&source).unwrap().get::<u16>("value").unwrap(), 8080);
    let mut target = Config::new();
    target.merge_properties_from_source(&source).unwrap();
    assert_eq!(target.get::<u16>("value").unwrap(), 8080);
    let composite = CompositeConfigSource::builder().add_source(source).build();
    assert_eq!(composite.load().unwrap().get::<u16>("value").unwrap(), 8080);
}

#[test]
fn test_managed_source_entry_points_enforce_local_budget() {
    let source = OverrideLoadSource {
        limits: SourceLimits::builder().max_properties(0).build(),
    };
    let mut target = Config::new();
    target.set("retained", true).unwrap();
    let before = target.clone();
    let top_error = Config::from_source(&source).unwrap_err();
    let merge_error = target.merge_properties_from_source(&source).unwrap_err();
    assert_eq!(target, before);
    let composite = CompositeConfigSource::builder().add_source(source).build();
    let composite_error = composite.load().unwrap_err();
    for error in [top_error, merge_error, composite_error] {
        assert_eq!(error.kind(), qubit_config::ConfigErrorKind::SourceLimitExceeded);
        assert_eq!(error.source_id(), Some("override:<memory>"));
        assert_eq!(error.source_budget_id(), Some("override:<memory>"));
    }
}

#[test]
fn test_managed_source_entry_points_propagate_errors_transactionally() {
    let source = FailingLoadSource;
    assert!(
        matches!(Config::from_source(&source), Err(ConfigError::PropertyNotFound(key)) if key == "expected-source-failure")
    );
    let mut target = Config::new();
    target.set("retained", true).unwrap();
    let before = target.clone();
    assert!(
        matches!(target.merge_properties_from_source(&source), Err(ConfigError::PropertyNotFound(key)) if key == "expected-source-failure")
    );
    assert_eq!(target, before);
    let composite = CompositeConfigSource::builder().add_source(source).build();
    assert!(matches!(composite.load(), Err(ConfigError::PropertyNotFound(key)) if key == "expected-source-failure"));
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

#[test]
fn test_source_load_context_set_final_contract() {
    let loaded = FinalSource {
        limits: SourceLimits::default(),
    }
    .load()
    .expect("final source should load after asserting expected errors");
    assert_eq!(loaded.get::<u16>("locked").unwrap(), 1);
    assert!(loaded.get_property("locked").unwrap().unwrap().is_final());
    assert_eq!(loaded.get::<u16>("unlocked").unwrap(), 3);
    assert!(!loaded.get_property("unlocked").unwrap().unwrap().is_final());
}

#[test]
fn test_source_load_context_set_final_does_not_recount_budget() {
    let loaded = FinalSource {
        limits: SourceLimits::builder().max_properties(1).max_nodes(1).build(),
    }
    .load()
    .expect("metadata operations must not spend property or node budget");
    assert!(loaded.get_property("locked").unwrap().unwrap().is_final());
}

#[test]
fn test_source_load_context_set_final_missing_under_zero_budget() {
    struct MissingFinalSource;
    impl ConfigSource for MissingFinalSource {
        fn source_id(&self) -> String {
            "missing:<memory>".to_owned()
        }
        fn limits(&self) -> SourceLimits {
            SourceLimits::builder().max_properties(0).max_nodes(0).build()
        }
        fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
            context.set_final("missing", true)
        }
    }

    assert!(matches!(MissingFinalSource.load(), Err(ConfigError::PropertyNotFound(key)) if key == "missing"));
}
