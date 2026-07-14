// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::error::Error;

use qubit_config::ConfigError;
use qubit_datatype::{DataConversionError, DataListConversionError, DataType, InvalidValueReason};
use qubit_value::ValueError;

fn invalid_integer() -> DataConversionError {
    DataConversionError::InvalidValue {
        from: DataType::String,
        to: DataType::Int32,
        reason: InvalidValueReason::InvalidSyntax {
            expected: "a base-10 integer",
        },
    }
}

#[test]
fn test_basic_error_messages() {
    assert_eq!(
        ConfigError::PropertyNotFound("server.port".to_string()).to_string(),
        "Property not found: server.port"
    );
    assert_eq!(
        ConfigError::PropertyHasNoValue("server.port".to_string()).to_string(),
        "Property 'server.port' has no value"
    );
}

#[test]
fn test_structured_conversion_error_is_redacted() {
    let error = ConfigError::ConversionError {
        key: "secret".to_string(),
        source_index: Some(2),
        source: invalid_integer(),
    };
    let message = error.to_string();
    assert!(message.contains("secret"));
    assert!(message.contains("invalid syntax"));
    assert!(!message.contains("hunter2"));
    assert!(error.source().is_some());
}

#[test]
fn test_data_conversion_missing_maps_to_no_value() {
    let error = ConfigError::from_data_conversion_error(
        "server.port",
        DataConversionError::Missing {
            from: DataType::String,
            to: DataType::Int32,
        },
    );
    assert!(matches!(
        error,
        ConfigError::PropertyHasNoValue(key) if key == "server.port"
    ));
}

#[test]
fn test_data_conversion_error_keeps_structure() {
    let error = ConfigError::from_data_conversion_error("server.port", invalid_integer());
    assert!(matches!(
        error,
        ConfigError::ConversionError {
            key,
            source_index: None,
            source: DataConversionError::InvalidValue {
                reason: InvalidValueReason::InvalidSyntax { .. },
                ..
            },
        } if key == "server.port"
    ));
}

#[test]
fn test_value_error_without_key() {
    let error = ConfigError::from(ValueError::TypeMismatch {
        expected: DataType::Int32,
        actual: DataType::String,
    });
    assert!(matches!(
        error,
        ConfigError::TypeMismatch { key, .. } if key.is_empty()
    ));

    let error = ConfigError::from(ValueError::DataConversion(invalid_integer()));
    assert!(matches!(
        error,
        ConfigError::ConversionError {
            key,
            source_index: None,
            ..
        } if key.is_empty()
    ));
}

#[test]
fn test_keyed_value_error_keeps_source_index() {
    let value_error = ValueError::DataListConversion(DataListConversionError {
        source_index: 4,
        source: invalid_integer(),
    });
    let error = ConfigError::from(("ports", value_error));
    assert!(matches!(
        error,
        ConfigError::ConversionError {
            key,
            source_index: Some(4),
            source: DataConversionError::InvalidValue { .. },
        } if key == "ports"
    ));
}

#[test]
fn test_keyed_value_error_variants() {
    assert!(matches!(
        ConfigError::from(("empty", ValueError::NoValue)),
        ConfigError::PropertyHasNoValue(key) if key == "empty"
    ));
    assert!(matches!(
        ConfigError::from((
            "port",
            ValueError::TypeMismatch {
                expected: DataType::Int32,
                actual: DataType::String,
            },
        )),
        ConfigError::TypeMismatch { key, .. } if key == "port"
    ));
}

#[test]
fn test_remaining_error_messages() {
    assert!(
        ConfigError::SubstitutionError("missing".to_string())
            .to_string()
            .contains("missing")
    );
    assert!(
        ConfigError::SubstitutionDepthExceeded(16)
            .to_string()
            .contains("16")
    );
    assert!(
        ConfigError::SubstitutionCycle {
            chain: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        }
        .to_string()
        .contains("a -> b -> a")
    );
    assert!(
        ConfigError::PropertyIsFinal("locked".to_string())
            .to_string()
            .contains("locked")
    );
    assert!(
        ConfigError::KeyConflict {
            path: "a.b".to_string(),
            existing: "scalar".to_string(),
            incoming: "object".to_string(),
        }
        .to_string()
        .contains("a.b")
    );
}
