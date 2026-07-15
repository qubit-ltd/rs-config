// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Focused tests for public configuration errors.

use qubit_config::ConfigError;
use qubit_datatype::{
    DataConversionError,
    DataType,
    InvalidValueReason,
};
use qubit_value::ValueError;

#[test]
fn test_config_error_maps_data_conversion_missing_with_key() {
    let error = ConfigError::from_data_conversion_error(
        "server.host",
        DataConversionError::Missing {
            from: DataType::String,
            to: DataType::String,
        },
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
        DataConversionError::InvalidValue {
            from: DataType::String,
            to: DataType::Bool,
            reason: InvalidValueReason::InvalidBoolean,
        },
    );

    assert!(matches!(
        &error,
        ConfigError::ConversionError {
            key,
            source_index: None,
            source: DataConversionError::InvalidValue {
                reason: InvalidValueReason::InvalidBoolean,
                ..
            },
        } if key == "server.enabled"
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
