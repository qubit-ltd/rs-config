// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::SourceLimitKind;
#[cfg(feature = "toml")]
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
#[cfg(feature = "env-file")]
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimits;
#[cfg(feature = "toml")]
use qubit_config::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use qubit_config::source::YamlConfigSource;

#[test]
fn properties_source_rejects_oversized_in_memory_input_before_loading() {
    let source = PropertiesConfigSource::builder()
        .content("key=abcd\n")
        .limits(SourceLimits::builder().max_input_bytes(3).build())
        .build();
    let _config = Config::new();

    let error = source.load().expect_err("oversized in-memory input must be rejected");
    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::InputBytes,
            limit: 3,
            observed_at_least: 9,
            ..
        }
    ));
}

#[test]
fn in_memory_source_debug_redacts_content_and_preserves_behavior() {
    const SECRET: &str = "SYNTHETIC_REVIEW_SECRET";
    let source = PropertiesConfigSource::from_content(format!("password={SECRET}"));
    let cloned = source.clone();
    for rendered in [format!("{source:?}"), format!("{source:#?}"), format!("{cloned:?}")] {
        assert!(!rendered.contains(SECRET), "Debug exposed source content: {rendered}");
        assert!(rendered.contains("bytes:"));
        assert!(rendered.contains("<redacted>"));
    }
    assert_eq!(source.load().unwrap().get::<String>("password").unwrap(), SECRET);

    #[cfg(feature = "env-file")]
    assert_redacted(&EnvFileConfigSource::from_content(format!("PASSWORD={SECRET}")), SECRET);
    #[cfg(feature = "toml")]
    assert_redacted(
        &TomlConfigSource::from_content(format!("password = '{SECRET}'")),
        SECRET,
    );
    #[cfg(feature = "yaml")]
    assert_redacted(&YamlConfigSource::from_content(format!("password: {SECRET}")), SECRET);

    let path = std::path::PathBuf::from("/tmp/private-config.properties");
    let file_source = PropertiesConfigSource::from_file(&path);
    assert!(format!("{file_source:?}").contains(path.to_str().unwrap()));
}

#[cfg(any(feature = "env-file", feature = "toml", feature = "yaml"))]
fn assert_redacted<T: std::fmt::Debug + Clone>(source: &T, secret: &str) {
    let cloned = source.clone();
    for rendered in [format!("{source:?}"), format!("{source:#?}"), format!("{cloned:?}")] {
        assert!(!rendered.contains(secret), "Debug exposed source content: {rendered}");
        assert!(rendered.contains("bytes:"));
        assert!(rendered.contains("<redacted>"));
    }
}

#[test]
fn properties_dotted_key_depth_remains_the_reference_behavior() {
    let source = PropertiesConfigSource::builder()
        .content("server.port=8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    assert!(matches!(
        source.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));

    let accepted = PropertiesConfigSource::builder()
        .content("server.port=8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(2).build())
        .build()
        .load()
        .expect("properties should accept a dotted key at its path depth");
    assert_eq!(accepted.get::<i64>("server.port").unwrap(), 8080);
}

#[cfg(feature = "toml")]
#[test]
fn test_toml_dotted_member_respects_output_depth() {
    let rejected = TomlConfigSource::builder()
        .content("\"server.port\" = 8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    assert!(matches!(
        rejected.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));

    let accepted = TomlConfigSource::builder()
        .content("\"server.port\" = 8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(2).build())
        .build()
        .load()
        .expect("a path at the configured depth should load");
    assert_eq!(accepted.get::<i64>("server.port").unwrap(), 8080);

    let rejected_mixed = TomlConfigSource::builder()
        .content("[server]\n\"db.port\" = 5432\n")
        .limits(SourceLimits::builder().max_nesting_depth(2).build())
        .build();
    assert!(matches!(
        rejected_mixed.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 2,
            observed_at_least: 3,
            ..
        })
    ));
    let accepted_mixed = TomlConfigSource::builder()
        .content("[server]\n\"db.port\" = 5432\n")
        .limits(SourceLimits::builder().max_nesting_depth(3).build())
        .build()
        .load()
        .expect("a mixed path at the configured depth should load");
    assert_eq!(accepted_mixed.get::<i64>("server.db.port").unwrap(), 5432);

    let _ = TomlConfigSource::builder()
        .content("")
        .limits(SourceLimits::builder().max_nesting_depth(0).build())
        .build()
        .load()
        .expect("an empty root table should load at depth zero");
    let _ = TomlConfigSource::builder()
        .content("[server]\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build()
        .load()
        .expect("an empty one-level table should load at depth one");
    let dotted_empty = TomlConfigSource::builder()
        .content("\"server.port\" = {}\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    assert!(matches!(
        dotted_empty.load(),
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
fn test_yaml_dotted_member_respects_output_depth() {
    let rejected = YamlConfigSource::builder()
        .content("\"server.port\": 8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    assert!(matches!(
        rejected.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));

    let accepted = YamlConfigSource::builder()
        .content("\"server.port\": 8080\n")
        .limits(SourceLimits::builder().max_nesting_depth(2).build())
        .build()
        .load()
        .expect("a path at the configured depth should load");
    assert_eq!(accepted.get::<i64>("server.port").unwrap(), 8080);

    let rejected_mixed = YamlConfigSource::builder()
        .content("server:\n  \"db.port\": 5432\n")
        .limits(SourceLimits::builder().max_nesting_depth(2).build())
        .build();
    assert!(matches!(
        rejected_mixed.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 2,
            observed_at_least: 3,
            ..
        })
    ));
    let accepted_mixed = YamlConfigSource::builder()
        .content("server:\n  \"db.port\": 5432\n")
        .limits(SourceLimits::builder().max_nesting_depth(3).build())
        .build()
        .load()
        .expect("a mixed path at the configured depth should load");
    assert_eq!(accepted_mixed.get::<i64>("server.db.port").unwrap(), 5432);

    let _ = YamlConfigSource::builder()
        .content("{}\n")
        .limits(SourceLimits::builder().max_nesting_depth(0).build())
        .build()
        .load()
        .expect("an empty root mapping should load at depth zero");
    let _ = YamlConfigSource::builder()
        .content("server: {}\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build()
        .load()
        .expect("an empty one-level mapping should load at depth one");
    let dotted_empty = YamlConfigSource::builder()
        .content("\"server.port\": {}\n")
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    assert!(matches!(
        dotted_empty.load(),
        Err(ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        })
    ));
}

#[cfg(feature = "toml")]
#[test]
fn strict_composite_depth_rejection_is_transactional() {
    let mut composite = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_nesting_depth(1).build())
        .build();
    composite.add(
        TomlConfigSource::builder()
            .content("\"server.port\" = 8080\n")
            .limits(SourceLimits::builder().max_nesting_depth(8).build())
            .build(),
    );

    let mut target = Config::new();
    target.set("existing", "kept").unwrap();
    let error = target
        .merge_properties_from_source(&composite)
        .expect_err("the composite depth limit must reject the child path");
    assert!(matches!(
        &error,
        ConfigError::SourceLimitExceeded {
            kind: SourceLimitKind::NestingDepth,
            limit: 1,
            observed_at_least: 2,
            ..
        }
    ));
    assert_eq!(error.source_budget_id(), Some("composite configuration source"));
    assert_eq!(target.get::<String>("existing").unwrap(), "kept");
    assert!(!target.contains("server.port").unwrap());
}
