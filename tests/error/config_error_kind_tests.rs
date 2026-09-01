// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for stable configuration error classification and context.

use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigPathViolation;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
use qubit_value::ValueError;

#[test]
fn test_config_error_exposes_stable_kind_and_path() {
    let error = ConfigError::from_data_conversion_error(
        "server.timeout",
        DataConversionError::invalid(
            DataType::String,
            DataType::Duration,
            InvalidValueReason::InvalidSyntax { expected: "a duration" },
        ),
    );

    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.timeout"));
    assert_eq!(error.source_index(), None);
}

#[test]
fn test_pathless_config_error_reports_no_path() {
    let error = ConfigError::Other("boom".to_string());

    assert_eq!(error.kind(), ConfigErrorKind::Other);
    assert_eq!(error.path(), None);
}

#[test]
fn test_source_errors_expose_source_id() {
    let io_error = ConfigError::SourceIoError {
        source_id: "config.toml".to_string(),
        source: std::io::Error::other("failed"),
    };
    assert_eq!(io_error.kind(), ConfigErrorKind::Io);
    assert_eq!(io_error.source_id(), Some("config.toml"));

    let parse_error = ConfigError::SourceParseError {
        source_id: "config.yaml".to_string(),
        path: None,
        source_index: None,
        message: "invalid YAML".to_string(),
    };
    assert_eq!(parse_error.kind(), ConfigErrorKind::Parse);
    assert_eq!(parse_error.source_id(), Some("config.yaml"));

    let source_path_error = ConfigError::SourceParseError {
        source_id: "config.yaml".to_string(),
        path: Some("servers".to_string()),
        source_index: Some(0),
        message: "mapping elements are unsupported".to_string(),
    };
    assert_eq!(source_path_error.path(), Some("servers"));
    assert_eq!(source_path_error.source_index(), Some(0));

    let unknown = ConfigError::UnknownProperties {
        paths: vec!["retry.extra".to_string()],
    };
    assert_eq!(unknown.kind(), ConfigErrorKind::UnknownProperty);
    assert_eq!(unknown.path(), Some("retry.extra"));
    assert_eq!(
        unknown.unknown_property_paths(),
        Some(["retry.extra".to_string()].as_slice())
    );
}

#[test]
fn test_config_error_kind_covers_every_public_variant() {
    let conversion = || {
        DataConversionError::invalid(
            DataType::String,
            DataType::Duration,
            InvalidValueReason::InvalidSyntax { expected: "a duration" },
        )
    };
    let cases = [
        (
            ConfigError::InvalidKey {
                key: "bad..key".to_string(),
                violation: ConfigPathViolation::EmptySegment,
            },
            ConfigErrorKind::InvalidKey,
        ),
        (
            ConfigError::InvalidPath {
                path: ".server".to_string(),
                violation: ConfigPathViolation::LeadingSeparator,
            },
            ConfigErrorKind::InvalidPath,
        ),
        (
            ConfigError::PropertyNotFound("key".to_string()),
            ConfigErrorKind::PropertyNotFound,
        ),
        (
            ConfigError::PropertyCandidatesNotFound {
                paths: vec!["first".to_string(), "second".to_string()],
            },
            ConfigErrorKind::PropertyNotFound,
        ),
        (
            ConfigError::PropertyHasNoValue("key".to_string()),
            ConfigErrorKind::PropertyHasNoValue,
        ),
        (
            ConfigError::TypeMismatch {
                key: "key".to_string(),
                expected: DataType::Int32,
                actual: DataType::String,
            },
            ConfigErrorKind::TypeMismatch,
        ),
        (
            ConfigError::ConversionError {
                key: "key".to_string(),
                source_index: Some(3),
                source: conversion(),
            },
            ConfigErrorKind::Conversion,
        ),
        (
            ConfigError::ValueError {
                key: "key".to_string(),
                source: ValueError::TypeMismatch {
                    expected: DataType::Int32,
                    actual: DataType::String,
                },
            },
            ConfigErrorKind::Value,
        ),
        (
            ConfigError::SubstitutionError {
                path: "key".to_string(),
                message: "failed".to_string(),
            },
            ConfigErrorKind::Substitution,
        ),
        (
            ConfigError::SubstitutionDepthExceeded {
                path: "key".to_string(),
                max_depth: 1,
            },
            ConfigErrorKind::SubstitutionDepthExceeded,
        ),
        (
            ConfigError::SubstitutionExpansionLimitExceeded {
                path: "key".to_string(),
                max_expansions: 1,
            },
            ConfigErrorKind::SubstitutionExpansionLimitExceeded,
        ),
        (
            ConfigError::SubstitutionOutputTooLarge {
                path: "key".to_string(),
                max_output_bytes: 1,
            },
            ConfigErrorKind::SubstitutionOutputTooLarge,
        ),
        (
            ConfigError::SubstitutionCycle {
                path: "key".to_string(),
                chain: vec!["key".to_string()],
            },
            ConfigErrorKind::SubstitutionCycle,
        ),
        (ConfigError::MergeError("failed".to_string()), ConfigErrorKind::Merge),
        (
            ConfigError::PropertyIsFinal("key".to_string()),
            ConfigErrorKind::PropertyIsFinal,
        ),
        (
            ConfigError::KeyConflict {
                source_id: None,
                path: "key".to_string(),
                existing: "key".to_string(),
                incoming: "key.child".to_string(),
            },
            ConfigErrorKind::KeyConflict,
        ),
        (
            ConfigError::UnknownProperties {
                paths: vec!["extra".to_string()],
            },
            ConfigErrorKind::UnknownProperty,
        ),
        (
            ConfigError::IoError(std::io::Error::other("failed")),
            ConfigErrorKind::Io,
        ),
        (ConfigError::ParseError("failed".to_string()), ConfigErrorKind::Parse),
        (
            ConfigError::DeserializeError {
                path: "key".to_string(),
                message: "failed".to_string(),
                source: None,
            },
            ConfigErrorKind::Deserialize,
        ),
        (ConfigError::Other("failed".to_string()), ConfigErrorKind::Other),
    ];

    for (error, expected_kind) in cases {
        assert_eq!(error.kind(), expected_kind, "unexpected error kind for {error}");
    }
}

#[test]
fn test_config_error_candidate_paths_retain_lookup_order() {
    let error = ConfigError::PropertyCandidatesNotFound {
        paths: vec!["primary".to_string(), "fallback".to_string()],
    };

    assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
    assert_eq!(error.path(), None);
    assert_eq!(
        error.candidate_paths(),
        Some(["primary".to_string(), "fallback".to_string()].as_slice()),
    );
}
