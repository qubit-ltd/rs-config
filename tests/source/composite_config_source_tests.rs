// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # `CompositeConfigSource` tests

#[cfg(feature = "toml")]
use std::path::PathBuf;
#[cfg(feature = "toml")]
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::CompositeConfigSourceBuilder;
use qubit_config::source::ConfigSource;
#[cfg(feature = "toml")]
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;
#[cfg(feature = "toml")]
use qubit_config::source::TomlConfigSource;

#[cfg(feature = "toml")]
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn load_source(config: &mut Config, source: &dyn ConfigSource) -> ConfigResult<()> {
    config.merge_properties_from_source(source)
}

struct CountingSource {
    calls: Arc<AtomicUsize>,
    fail: bool,
    value: u16,
}

struct FinalSource;

impl ConfigSource for FinalSource {
    fn source_id(&self) -> String {
        "final:<memory>".to_owned()
    }
    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("locked", 1_u16)?;
        context.set_final("locked", true)
    }
}

impl ConfigSource for CountingSource {
    fn source_id(&self) -> String {
        "test:counting-source".to_owned()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default()
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(ConfigError::PropertyNotFound("counting-source-failure".to_owned()));
        }
        context.set("value", self.value)
    }
}

#[cfg(feature = "toml")]
fn run_with_env(name: &str, key: &str, value: &str) -> bool {
    const CHILD: &str = "RS_CONFIG_COMPOSITE_ENV_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(name)
        .arg("--nocapture")
        .env_clear()
        .env(CHILD, "1")
        .env(key, value);
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let output = command.output().expect("child test process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
    false
}

#[test]
fn composite_builder_default_and_consuming_add_source_preserve_order() {
    let default_builder: fn() -> CompositeConfigSourceBuilder = Default::default;
    let builder = std::hint::black_box(default_builder)();
    let composite = std::hint::black_box(CompositeConfigSourceBuilder::add_source::<PropertiesConfigSource>)(
        builder,
        PropertiesConfigSource::from_content("answer=42\n"),
    )
    .build();

    assert_eq!(composite.len(), 1);
    assert_eq!(composite.load().unwrap().get::<u8>("answer").unwrap(), 42);
}

// ============================================================================
// CompositeConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_composite_config_source {
    use qubit_budget::BudgetError;

    use super::Arc;
    use super::AtomicUsize;
    use super::CompositeConfigSource;
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::CountingSource;
    #[cfg(feature = "toml")]
    use super::EnvConfigSource;
    use super::FinalSource;
    use super::Ordering;
    use super::PropertiesConfigSource;
    use super::SourceLimitKind;
    use super::SourceLimits;
    #[cfg(feature = "toml")]
    use super::TomlConfigSource;
    #[cfg(feature = "toml")]
    use super::fixture;
    use super::load_source;
    #[cfg(feature = "toml")]
    use super::run_with_env;

    #[test]
    fn test_new_composite_is_empty() {
        let composite = CompositeConfigSource::new();
        assert!(composite.is_empty());
        assert_eq!(composite.len(), 0);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_add_source_increases_len() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        assert_eq!(composite.len(), 1);
        assert!(!composite.is_empty());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_add_multiple_sources() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(PropertiesConfigSource::from_file(fixture("basic.properties")));
        assert_eq!(composite.len(), 2);
    }

    #[cfg(feature = "toml")]
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
        let calls = [
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        ];
        let mut composite = CompositeConfigSource::new();
        composite.add(CountingSource {
            calls: Arc::clone(&calls[0]),
            fail: true,
            value: 1,
        });
        composite.add(CountingSource {
            calls: Arc::clone(&calls[1]),
            fail: false,
            value: 2,
        });
        composite.add(CountingSource {
            calls: Arc::clone(&calls[2]),
            fail: false,
            value: 3,
        });

        let error = composite.load().expect_err("the first source should fail");
        assert!(matches!(error, ConfigError::PropertyNotFound(ref path) if path == "counting-source-failure"));
        assert_eq!(error.path(), Some("counting-source-failure"));
        assert_eq!(calls.map(|count| count.load(Ordering::SeqCst)), [1, 0, 0]);
    }

    #[test]
    fn test_default_creates_empty_composite() {
        let composite = CompositeConfigSource::default();
        assert!(composite.is_empty());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_composite_with_env_override() {
        if !run_with_env(
            "test_composite_config_source::test_composite_with_env_override",
            "CTEST_HOST",
            "env-host",
        ) {
            return;
        }

        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(EnvConfigSource::from_prefix("CTEST_"));

        let mut config = Config::new();
        load_source(&mut config, &composite).unwrap();

        // env source overrides toml
        assert_eq!(config.get::<String>("host").unwrap(), "env-host");
        // toml-only keys still present
        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_merge_from_composite_config_source() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        config.merge_properties_from_source(&composite).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "production-server");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_add_returns_mutable_ref_for_chaining() {
        // Verify the builder-style chaining works
        let mut composite = CompositeConfigSource::new();
        composite
            .add(TomlConfigSource::from_file(fixture("basic.toml")))
            .add(TomlConfigSource::from_file(fixture("override.toml")));

        assert_eq!(composite.len(), 2);
    }

    #[cfg(feature = "toml")]
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
    fn test_composite_preserves_final_and_rejects_later_override_transactionally() {
        let composite = CompositeConfigSource::builder()
            .add_source(FinalSource)
            .add_source(PropertiesConfigSource::from_content("locked=2\n"))
            .build();

        let mut target = Config::new();
        target.set("kept", 7_u16).unwrap();
        let before = target.clone();
        let error = target
            .merge_properties_from_source(&composite)
            .expect_err("later source must not override a final property");
        assert!(matches!(error, ConfigError::PropertyIsFinal(ref key) if key == "locked"));
        assert_eq!(target, before);

        let only_final = CompositeConfigSource::builder().add_source(FinalSource).build();
        let mut loaded = only_final.load().expect("final source should load");
        assert!(loaded.get_property("locked").unwrap().unwrap().is_final());
        let error = loaded
            .set("locked", 3_u16)
            .expect_err("the final marker must survive composite loading");
        assert!(matches!(error, ConfigError::PropertyIsFinal(ref key) if key == "locked"));
    }

    #[test]
    fn test_composite_enforces_aggregate_property_budget() {
        let mut composite = CompositeConfigSource::builder()
            .limits(SourceLimits::builder().max_properties(1).build())
            .build();
        composite.add(PropertiesConfigSource::from_content("first=1\n"));
        composite.add(PropertiesConfigSource::from_content("second=2\n"));

        let error = composite
            .load()
            .expect_err("the second property should exceed the aggregate budget");

        assert_eq!(error.source_budget_id(), Some("composite configuration source"));
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
        let calls = [
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        ];
        let mut composite = CompositeConfigSource::builder()
            .limits(SourceLimits::builder().max_sources(1).build())
            .build();
        for (index, call_count) in calls.iter().enumerate() {
            composite.add(CountingSource {
                calls: Arc::clone(call_count),
                fail: false,
                value: index as u16 + 1,
            });
        }

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
        assert_eq!(calls.map(|count| count.load(Ordering::SeqCst)), [1, 0, 0]);
    }

    #[test]
    fn test_composite_zero_source_limit_rejects_before_loading_any_source() {
        let calls = [
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        ];
        let mut composite = CompositeConfigSource::builder()
            .limits(SourceLimits::builder().max_sources(0).build())
            .build();
        for (index, call_count) in calls.iter().enumerate() {
            composite.add(CountingSource {
                calls: Arc::clone(call_count),
                fail: false,
                value: index as u16 + 1,
            });
        }

        let error = composite
            .load()
            .expect_err("zero allowed sources should reject the first source");
        assert!(matches!(
            error.budget_error(),
            Some(BudgetError::Insufficient {
                resource: SourceLimitKind::SourceCount,
                limit: 0,
                remaining: 0,
                requested: 1,
            })
        ));
        assert_eq!(calls.map(|count| count.load(Ordering::SeqCst)), [0, 0, 0]);
    }

    #[test]
    fn test_composite_loads_every_successful_source_once_in_order() {
        let calls = [
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        ];
        let mut composite = CompositeConfigSource::new();
        for (index, call_count) in calls.iter().enumerate() {
            composite.add(CountingSource {
                calls: Arc::clone(call_count),
                fail: false,
                value: index as u16 + 1,
            });
        }

        let config = composite.load().expect("all counting sources should succeed");
        assert_eq!(calls.map(|count| count.load(Ordering::SeqCst)), [1, 1, 1]);
        assert_eq!(config.get::<u16>("value").unwrap(), 3);
    }
}
