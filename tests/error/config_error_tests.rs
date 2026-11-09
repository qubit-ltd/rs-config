// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Focused tests for public configuration errors.

use std::num::NonZeroUsize;

use qubit_budget::BudgetError;
use qubit_budget::Observation;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigPathViolation;
use qubit_config::SourceLocation;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
use qubit_value::ValueError;

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
