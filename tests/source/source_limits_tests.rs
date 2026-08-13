// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for bounded configuration source ingestion.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
#[cfg(feature = "env-file")]
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;
#[cfg(feature = "toml")]
use qubit_config::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use qubit_config::source::YamlConfigSource;

struct InputAccountingSource {
    amount: usize,
}

impl ConfigSource for InputAccountingSource {
    fn source_id(&self) -> String {
        "input-accounting".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::default().with_max_input_bytes(5)
    }

    fn load_into(
        &self,
        context: &mut SourceLoadContext<'_>,
    ) -> ConfigResult<()> {
        context.consume_input_bytes(self.amount)
    }
}

#[test]
fn source_context_charges_local_and_aggregate_budgets_atomically() {
    let mut composite = CompositeConfigSource::new()
        .with_limits(SourceLimits::default().with_max_input_bytes(3));
    composite.add(InputAccountingSource { amount: 2 });
    composite.add(InputAccountingSource { amount: 2 });

    let error = composite
        .load()
        .expect_err("the aggregate budget should reject the second charge");

    assert_eq!(error.source_id(), Some("input-accounting"));
    assert_eq!(
        error.source_budget_id(),
        Some("composite configuration source")
    );
    assert_eq!(
        error.budget_error().and_then(|error| error.remaining()),
        Some(1)
    );
}

#[test]
fn source_limits_are_bounded_by_default_and_can_be_unbounded() {
    let limits = SourceLimits::default();
    assert_eq!(limits.max_input_bytes(), 8 * 1024 * 1024);
    assert_eq!(limits.max_properties(), 65_536);
    assert_eq!(limits.max_nodes(), 262_144);
    assert_eq!(limits.max_sources(), 256);
    assert_eq!(limits.max_nesting_depth(), 64);

    let limits = SourceLimits::unbounded();
    assert_eq!(limits.max_input_bytes(), usize::MAX);
    assert_eq!(limits.max_properties(), usize::MAX);
    assert_eq!(limits.max_nodes(), usize::MAX);
    assert_eq!(limits.max_sources(), usize::MAX);
    assert_eq!(limits.max_nesting_depth(), usize::MAX);

    let zero = SourceLimits::default()
        .with_max_input_bytes(0)
        .with_max_properties(0)
        .with_max_nodes(0)
        .with_max_sources(0)
        .with_max_nesting_depth(0);
    assert_eq!(zero.max_input_bytes(), 0);
    assert_eq!(zero.max_properties(), 0);
    assert_eq!(zero.max_nodes(), 0);
    assert_eq!(zero.max_sources(), 0);
    assert_eq!(zero.max_nesting_depth(), 0);
}

#[test]
fn source_budget_failed_charge_preserves_remaining_capacity() {
    let source = InputAccountingSource { amount: 6 };
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::InputBytes,
            limit: 5,
            observed_at_least: 6,
            ..
        })
    ));
}

/// Verifies an exact nesting-depth boundary is accepted before the next level
/// is rejected with the configured source-limit facts.
#[test]
fn source_budget_nesting_depth_accepts_limit_and_rejects_next_level() {
    let source = PropertiesConfigSource::from_content("a=1\n")
        .with_limits(SourceLimits::default().with_max_nesting_depth(0));
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 0,
            ..
        })
    ));
}

#[test]
fn properties_source_rejects_oversized_input_transactionally() {
    let source = PropertiesConfigSource::from_content("a=1\n")
        .with_limits(SourceLimits::default().with_max_input_bytes(3));
    let mut config = Config::new();
    config.set("existing", "kept").unwrap();

    let result = config.merge_properties_from_source(&source);

    assert!(matches!(
        result,
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::InputBytes,
            limit: 3,
            observed_at_least: 4,
            ..
        })
    ));
    assert_eq!(config.get::<String>("existing").unwrap(), "kept");
    assert_eq!(config.len(), 1);
}

#[test]
fn properties_file_source_reports_file_identity_for_limits() {
    let dir = tempfile::tempdir().expect("temporary directory should exist");
    let path = dir.path().join("limited.properties");
    std::fs::write(&path, "a=1\n").expect("properties file should be written");
    let source = PropertiesConfigSource::from_file(&path)
        .with_limits(SourceLimits::default().with_max_input_bytes(3));

    let error = source
        .load()
        .expect_err("file input should exceed the configured limit");
    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded { source_id, .. }
            if source_id == path.display().to_string()
    ));
}

#[test]
fn properties_source_counts_duplicate_assignments() {
    let source = PropertiesConfigSource::from_content("same=1\nsame=2\n")
        .with_limits(SourceLimits::default().with_max_properties(1));

    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::PropertyCount,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
}

/// Verifies a property-budget failure leaves the target configuration intact.
#[test]
fn properties_source_property_budget_failure_is_transactional() {
    let source = PropertiesConfigSource::from_content("first=1\nsecond=2\n")
        .with_limits(SourceLimits::default().with_max_properties(1));
    let mut config = Config::new();
    config
        .set("existing", "kept")
        .expect("the existing property should be set");

    let result = config.merge_properties_from_source(&source);

    assert!(matches!(
        result,
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::PropertyCount,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
    assert_eq!(config.len(), 1);
    assert_eq!(
        config
            .get::<String>("existing")
            .expect("the existing property should remain readable"),
        "kept",
    );
}

#[test]
fn properties_source_rejects_invalid_keys_and_excessive_key_depth() {
    assert!(matches!(
        PropertiesConfigSource::from_content("bad..key=1\n").load(),
        Err(ConfigError::SourceParseError {
            source_id,
            path: Some(path),
            ..
        }) if source_id == "properties:<memory>" && path == "bad..key"
    ));

    let deep_key = std::iter::repeat_n("a", 65).collect::<Vec<_>>().join(".");
    let source = PropertiesConfigSource::from_content(format!("{deep_key}=1"))
        .with_limits(SourceLimits::default().with_max_nesting_depth(64));
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 64,
            observed_at_least: 65,
            ..
        })
    ));
}

#[cfg(feature = "env-file")]
#[test]
fn env_file_memory_source_enforces_property_budget() {
    let source = EnvFileConfigSource::from_content("FIRST=1\nSECOND=2\n")
        .with_limits(SourceLimits::default().with_max_properties(1));
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::PropertyCount,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
}

#[cfg(feature = "toml")]
#[test]
fn toml_source_rejects_excessive_nesting_depth() {
    let source = TomlConfigSource::from_content("[server]\nport = 8080\n")
        .with_limits(SourceLimits::default().with_max_nesting_depth(1));
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
}

#[cfg(feature = "yaml")]
#[test]
fn yaml_source_rejects_excessive_nesting_depth() {
    let source = YamlConfigSource::from_content("server:\n  port: 8080\n")
        .with_limits(SourceLimits::default().with_max_nesting_depth(1));
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
}
