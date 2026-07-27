// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Focused tests for public configuration errors.

use qubit_config::{
    ConfigError,
    ConfigErrorKind,
    ConfigPathViolation,
};
use qubit_datatype::{
    DataConversionError,
    DataType,
    InvalidValueReason,
};
use qubit_value::ValueError;

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
        ConfigError::PropertyHasNoValue(key) if key == "server.host"
    ));
}

#[test]
fn test_config_error_retains_structured_failure() {
    let error = ConfigError::from_data_conversion_error(
        "server.enabled",
        DataConversionError::invalid(
            DataType::String,
            DataType::Bool,
            InvalidValueReason::InvalidBoolean,
        ),
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
