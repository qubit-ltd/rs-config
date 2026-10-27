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

fn assert_redacted<T: std::fmt::Debug + Clone>(source: &T, secret: &str) {
    let cloned = source.clone();
    for rendered in [format!("{source:?}"), format!("{source:#?}"), format!("{cloned:?}")] {
        assert!(!rendered.contains(secret), "Debug exposed source content: {rendered}");
        assert!(rendered.contains("bytes:"));
        assert!(rendered.contains("<redacted>"));
    }
}
