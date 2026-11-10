// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Focused tests for public configuration errors.

use std::error::Error;
use std::num::NonZeroUsize;

use qubit_budget::BudgetError;
use qubit_budget::Observation;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigPathViolation;
use qubit_config::ConfigReader;
use qubit_config::ConfigSource;
use qubit_config::SourceLocation;
use qubit_config::source::PropertiesConfigSource;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
use qubit_value::ValueError;

#[test]
fn test_missing_property_display_escapes_controls_and_keeps_path() {
    let key = "a\n\r\t\u{1b}\0\u{7f}\u{85}雪";
    let error = qubit_config::Config::new().get::<String>(key).unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(error.path(), Some(key));
    assert_eq!(error.candidate_paths().unwrap(), &[key.to_owned()]);
    let displayed = error.to_string();
    assert!(!displayed.chars().any(char::is_control));
    assert!(displayed.contains(r"a\n\r\t\u{1b}\0\u{7f}\u{85}雪"));
}

#[test]
fn test_ordinary_display_preserves_visible_text_and_literal_backslashes() {
    let key = r#"雪["quoted"]\n"#;
    let error = ConfigError::PropertyNotFound(key.to_string());
    assert_eq!(error.to_string(), format!("Property not found: {key}"));
}

#[test]
fn test_config_error_display_escapes_dynamic_diagnostics_and_preserves_fields() {
    let noisy = "x\n\r\t\u{1b}\0\u{7f}\u{85}雪";
    let variants = vec![
        ConfigError::InvalidKey {
            key: noisy.into(),
            violation: ConfigPathViolation::EmptySegment,
        },
        ConfigError::SourceInvalidKey {
            source_id: noisy.into(),
            key: noisy.into(),
            violation: ConfigPathViolation::EmptySegment,
        },
        ConfigError::InvalidPath {
            path: noisy.into(),
            violation: ConfigPathViolation::EmptySegment,
        },
        ConfigError::SourceLimitExceeded {
            source_id: noisy.into(),
            budget_id: noisy.into(),
            kind: qubit_config::source::SourceLimitKind::InputBytes,
            limit: 1,
            observed_at_least: 2,
            source: BudgetError::LimitExceeded {
                resource: qubit_config::source::SourceLimitKind::InputBytes,
                observed: Observation::Exact(2),
                maximum: 1,
            },
        },
        ConfigError::PropertyCandidatesNotFound {
            paths: vec![noisy.into(), "second\n".into()],
        },
        ConfigError::PropertyHasNoValue(noisy.into()),
        ConfigError::TypeMismatch {
            key: noisy.into(),
            expected: DataType::String,
            actual: DataType::Bool,
        },
        ConfigError::ConversionError {
            key: noisy.into(),
            source_index: Some(2),
            source: DataConversionError::missing(DataType::String, DataType::String),
        },
        ConfigError::PlatformIntegerOutOfRange {
            key: noisy.into(),
            source_index: None,
        },
        ConfigError::ValueError {
            key: noisy.into(),
            source: ValueError::TypeMismatch {
                expected: DataType::String,
                actual: DataType::Bool,
            },
        },
        ConfigError::SubstitutionError {
            path: noisy.into(),
            message: noisy.into(),
        },
        ConfigError::SubstitutionDepthExceeded {
            path: noisy.into(),
            max_depth: 1,
        },
        ConfigError::SubstitutionExpansionLimitExceeded {
            path: noisy.into(),
            max_expansions: 1,
        },
        ConfigError::SubstitutionOutputTooLarge {
            path: noisy.into(),
            max_output_bytes: 1,
        },
        ConfigError::SubstitutionCycle {
            path: noisy.into(),
            chain: vec!["a\n".into(), "b\r".into()],
        },
        ConfigError::MergeError(noisy.into()),
        ConfigError::PropertyIsFinal(noisy.into()),
        ConfigError::SourcePropertyIsFinal {
            source_id: noisy.into(),
            key: noisy.into(),
        },
        ConfigError::UnknownProperties {
            paths: vec![noisy.into(), "second\n".into()],
        },
        ConfigError::IoError(std::io::Error::other(noisy)),
        ConfigError::SourceIoError {
            source_id: noisy.into(),
            source: std::io::Error::other(noisy),
        },
        ConfigError::ParseError(noisy.into()),
        ConfigError::SourceParseError {
            source_id: noisy.into(),
            path: Some(noisy.into()),
            source_index: None,
            message: noisy.into(),
            location: None,
        },
        ConfigError::DeserializeError {
            path: noisy.into(),
            message: noisy.into(),
            source: None,
        },
        ConfigError::Other(noisy.into()),
    ];
    for error in variants {
        let rendered = error.to_string();
        assert!(!rendered.chars().any(char::is_control), "{error:?}: {rendered:?}");
    }

    let candidates = ConfigError::PropertyCandidatesNotFound {
        paths: vec!["first\n".into(), "second\r".into()],
    };
    assert_eq!(
        candidates.to_string(),
        r"None of the candidate properties was found: first\n, second\r"
    );
    assert_eq!(candidates.candidate_paths().unwrap(), &["first\n", "second\r"]);
    let unknown = ConfigError::UnknownProperties {
        paths: vec!["one\n".into(), "two\r".into()],
    };
    assert_eq!(unknown.to_string(), r"Unknown configuration properties: one\n, two\r");
    assert_eq!(unknown.unknown_property_paths().unwrap(), &["one\n", "two\r"]);
    let indexed = ConfigError::IndexedSubstitutionError {
        source_index: 7,
        source: Box::new(ConfigError::SubstitutionError {
            path: noisy.into(),
            message: noisy.into(),
        }),
    };
    assert!(!indexed.to_string().chars().any(char::is_control));
    assert_eq!(indexed.source_index(), Some(7));
    assert_eq!(indexed.kind(), ConfigErrorKind::Substitution);
    assert!(Error::source(&indexed).is_some());

    let io = ConfigError::SourceIoError {
        source_id: "source".into(),
        source: std::io::Error::other(noisy),
    };
    let source = Error::source(&io).unwrap();
    assert_eq!(source.to_string(), noisy);
    assert!(!io.to_string().chars().any(char::is_control));
}

#[test]
fn test_source_diagnostics_escape_labels_and_final_conflict_text() {
    let label = "source\n\u{1b}雪";
    let source = PropertiesConfigSource::builder()
        .label(label)
        .content("a..b=value")
        .build();
    let invalid = source.load().unwrap_err();
    assert_eq!(invalid.source_id(), Some(label));
    assert_eq!(invalid.path(), Some("a..b"));
    assert!(!invalid.to_string().chars().any(char::is_control));

    let mut target = qubit_config::Config::new();
    target.set("locked", "old").unwrap();
    target.set_final("locked", true).unwrap();
    let incoming = PropertiesConfigSource::builder()
        .label(label)
        .content("locked=new")
        .build();
    let conflict = target.merge_properties_from_source(&incoming).unwrap_err();
    assert_eq!(conflict.source_id(), Some(label));
    assert_eq!(conflict.path(), Some("locked"));
    assert!(!conflict.to_string().chars().any(char::is_control));
    assert!(conflict.to_string().contains(r"source\n\u{1b}雪"));
}

#[test]
fn test_missing_candidates_display_keeps_control_character_path_order() {
    let first = "first\nkey";
    let second = "second\tkey";
    let error = qubit_config::Config::new()
        .get_any::<String>([first, second])
        .unwrap_err();

    assert_eq!(error.candidate_paths().unwrap(), &[first.to_owned(), second.to_owned()]);
    assert_eq!(
        error.to_string(),
        r"None of the candidate properties was found: first\nkey, second\tkey"
    );
    assert!(!error.to_string().chars().any(char::is_control));
}

#[test]
fn test_ensure_known_keys_keeps_raw_unknown_paths() {
    let first = "first\nkey";
    let second = "second\tkey";
    let mut config = qubit_config::Config::new();
    config.set(first, "one").unwrap();
    config.set(second, "two").unwrap();

    let error = config.ensure_known_keys(&[], &[]).unwrap_err();
    assert_eq!(
        error.unknown_property_paths().unwrap(),
        &[first.to_owned(), second.to_owned()]
    );
    assert_eq!(
        error.to_string(),
        r"Unknown configuration properties: first\nkey, second\tkey"
    );
    assert!(!error.to_string().chars().any(char::is_control));
}

#[test]
fn test_source_parse_location_preserves_existing_context() {
    let location = SourceLocation::new(NonZeroUsize::new(2).unwrap(), None);
    let line: fn(&SourceLocation) -> usize = SourceLocation::line;
    let column: fn(&SourceLocation) -> Option<usize> = SourceLocation::column;
    assert_eq!(line(&location), 2);
    assert_eq!(column(&location), None);

    let error = ConfigError::SourceParseError {
        source_id: "baseline".into(),
        path: Some("a..b".into()),
        source_index: Some(3),
        message: "invalid syntax".into(),
        location: Some(location),
    };
    assert_eq!(error.kind(), ConfigErrorKind::Parse);
    assert_eq!(error.path(), Some("a..b"));
    assert_eq!(error.source_id(), Some("baseline"));
    assert_eq!(error.source_index(), Some(3));
    assert_eq!(error.source_location(), Some(location));

    assert_eq!(ConfigError::ParseError("bad".into()).source_location(), None);
    let ordinary_parse = ConfigError::SourceParseError {
        source_id: "plain".into(),
        path: None,
        source_index: None,
        message: "bad".into(),
        location: None,
    };
    assert_eq!(ordinary_parse.source_location(), None);

    let budget_error = ConfigError::SourceLimitExceeded {
        source_id: "memory:limited".into(),
        budget_id: "memory:limited".into(),
        kind: qubit_config::source::SourceLimitKind::InputBytes,
        limit: 1,
        observed_at_least: 2,
        source: BudgetError::LimitExceeded {
            resource: qubit_config::source::SourceLimitKind::InputBytes,
            observed: Observation::Exact(2),
            maximum: 1,
        },
    };
    assert_eq!(budget_error.source_location(), None);
}

#[test]
fn test_invalid_key_and_path_errors_keep_structured_context() {
    let key_error = ConfigError::InvalidKey {
        key: "bad..key".to_string(),
        violation: ConfigPathViolation::EmptySegment,
    };
    assert_eq!(key_error.kind(), ConfigErrorKind::InvalidKey);
    assert_eq!(key_error.path(), Some("bad..key"));
    assert!(key_error.to_string().contains("empty segment"));

    let path_error = ConfigError::InvalidPath {
        path: ".server".to_string(),
        violation: ConfigPathViolation::LeadingSeparator,
    };
    assert_eq!(path_error.kind(), ConfigErrorKind::InvalidPath);
    assert_eq!(path_error.path(), Some(".server"));
    assert!(path_error.to_string().contains("starts with a separator"));
}

#[test]
fn test_config_error_maps_data_conversion_missing_with_key() {
    let error = ConfigError::from_data_conversion_error(
        "server.host",
        DataConversionError::missing(DataType::String, DataType::String),
    );

    assert!(matches!(
        &error,
        ConfigError::ValueError { key, source } if key == "server.host" && source.missing().unwrap().is_conversion()
    ));
}

#[test]
fn test_config_error_retains_structured_failure() {
    let error = ConfigError::from_data_conversion_error(
        "server.enabled",
        DataConversionError::invalid(DataType::String, DataType::Bool, InvalidValueReason::InvalidBoolean),
    );

    assert!(matches!(
        &error,
        ConfigError::ConversionError {
            key,
            source_index: None,
            source,
        } if key == "server.enabled"
            && matches!(
                source.reason(),
                Some(InvalidValueReason::InvalidBoolean),
            )
    ));
}

#[test]
fn test_platform_integer_range_error_exposes_context_without_value() {
    let error = ConfigError::PlatformIntegerOutOfRange {
        key: "server.secret_limit".to_string(),
        source_index: Some(0),
    };

    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.secret_limit"));
    assert_eq!(error.source_index(), Some(0));
    assert!(error.to_string().contains("server.secret_limit"));
    assert!(!error.to_string().contains("18446744073709551616"));
    assert!(!format!("{error:?}").contains("18446744073709551616"));
}

#[test]
fn test_config_value_error_fallback_retains_key_and_source() {
    let error = ConfigError::ValueError {
        key: "server.port".to_string(),
        source: ValueError::TypeMismatch {
            expected: DataType::UInt16,
            actual: DataType::String,
        },
    };

    assert!(error.to_string().contains("server.port"));
    assert!(std::error::Error::source(&error).is_some());
    assert!(matches!(
        error,
        ConfigError::ValueError {
            key,
            source: ValueError::TypeMismatch {
                expected: DataType::UInt16,
                actual: DataType::String,
            },
        } if key == "server.port"
    ));
}

#[test]
fn test_config_error_optional_context_accessors_are_callable_as_functions() {
    let candidates = ConfigError::PropertyCandidatesNotFound {
        paths: vec!["primary".to_string(), "fallback".to_string()],
    };
    let candidate_paths: fn(&ConfigError) -> Option<&[String]> = ConfigError::candidate_paths;
    assert_eq!(
        std::hint::black_box(candidate_paths)(&candidates),
        Some(["primary".to_string(), "fallback".to_string()].as_slice()),
    );

    let conversion = ConfigError::ConversionError {
        key: "items".to_string(),
        source_index: Some(2),
        source: DataConversionError::invalid(DataType::String, DataType::Bool, InvalidValueReason::InvalidBoolean),
    };
    let source_index: fn(&ConfigError) -> Option<usize> = ConfigError::source_index;
    assert_eq!(std::hint::black_box(source_index)(&conversion), Some(2));
}

#[test]
fn test_single_missing_candidate_is_exposed_as_a_candidate_path() {
    let error = ConfigError::PropertyNotFound("server.port".to_string());
    assert_eq!(error.candidate_paths(), Some(["server.port".to_string()].as_slice()),);
}

#[test]
fn test_error_accessors_retain_missing_value_source_and_conflict_context() {
    let no_value = ConfigError::PropertyHasNoValue("server.port".to_string());
    assert_eq!(no_value.path(), Some("server.port"));

    let source_parse = ConfigError::SourceParseError {
        source_id: "memory:test".to_string(),
        path: Some("items".to_string()),
        source_index: Some(3),
        message: "invalid element".to_string(),
        location: None,
    };
    assert_eq!(source_parse.source_index(), Some(3));

    let conflict = ConfigError::KeyConflict {
        source_id: Some("memory:defaults".to_string()),
        path: "server.port".to_string(),
        existing: "integer".to_string(),
        incoming: "string".to_string(),
    };
    assert_eq!(conflict.source_id(), Some("memory:defaults"));
}

#[test]
fn test_key_conflict_display_escapes_control_characters_and_keeps_fields() {
    let path = "foo\nbar\r\u{1b}\t\\雪";
    let existing = "old\n\r\u{1b}\t\\值";
    let incoming = "new\n\r\u{1b}\t\\値";
    let error = ConfigError::KeyConflict {
        source_id: Some("memory:test".to_string()),
        path: path.to_string(),
        existing: existing.to_string(),
        incoming: incoming.to_string(),
    };

    let rendered = error.to_string();
    assert_eq!(
        rendered,
        r"Configuration key conflict at 'foo\nbar\r\u{1b}\t\\雪': existing old\n\r\u{1b}\t\\值, incoming new\n\r\u{1b}\t\\値"
    );
    for control in ['\n', '\r', '\u{1b}', '\t'] {
        assert!(!rendered.contains(control));
    }
    assert!(rendered.contains(r"foo\nbar\r\u{1b}\t\\雪"));
    assert!(rendered.contains(r"old\n\r\u{1b}\t\\值"));
    assert!(rendered.contains(r"new\n\r\u{1b}\t\\値"));
    assert!(matches!(
        &error,
        ConfigError::KeyConflict { path: actual_path, existing: actual_existing, incoming: actual_incoming, .. }
            if actual_path == path && actual_existing == existing && actual_incoming == incoming
    ));

    let ascii = ConfigError::KeyConflict {
        source_id: None,
        path: "foo.bar".to_string(),
        existing: "old key".to_string(),
        incoming: "new key".to_string(),
    };
    assert_eq!(
        ascii.to_string(),
        "Configuration key conflict at 'foo.bar': existing old key, incoming new key"
    );
}

#[test]
fn test_context_accessors_cover_empty_and_indexed_cases() {
    let no_candidates = ConfigError::PropertyCandidatesNotFound { paths: Vec::new() };
    assert_eq!(no_candidates.path(), None);

    let multiple_unknown = ConfigError::UnknownProperties {
        paths: vec!["first".to_string(), "second".to_string()],
    };
    assert_eq!(multiple_unknown.path(), None);

    let indexed_parse = ConfigError::SourceParseError {
        source_id: "memory:test".to_string(),
        path: None,
        source_index: Some(2),
        message: "invalid element at source index 2".to_string(),
        location: None,
    };
    assert_eq!(indexed_parse.path(), None);
    assert_eq!(indexed_parse.source_index(), Some(2));

    let invalid_value = ConfigError::from_data_conversion_error(
        "value",
        DataConversionError::invalid(DataType::String, DataType::Bool, InvalidValueReason::InvalidBoolean),
    );
    assert_eq!(invalid_value.source_index(), None);
}
