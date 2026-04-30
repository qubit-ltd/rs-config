/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # ConfigError Unit Tests
//!
//! Tests all error types and conversions of the ConfigError enum.

use qubit_common::DataType;
use qubit_config::{Config, ConfigError};
use qubit_value::ValueError;
use std::io;

// ============================================================================
// Basic Error Type Tests
// ============================================================================

#[test]
fn test_property_not_found_error() {
    let error = ConfigError::PropertyNotFound("test.property".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Property not found"));
    assert!(error_msg.contains("test.property"));
}

#[test]
fn test_property_has_no_value_error() {
    let error = ConfigError::PropertyHasNoValue("test.property".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("has no value"));
    assert!(error_msg.contains("test.property"));
}

#[test]
fn test_type_mismatch_error() {
    let error = ConfigError::TypeMismatch {
        key: "server.port".to_string(),
        expected: DataType::Int32,
        actual: DataType::String,
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Type mismatch"));
    assert!(error_msg.contains("expected"));
    assert!(error_msg.contains("actual"));
    assert!(error_msg.contains("server.port"));
}

#[test]
fn test_conversion_error() {
    let error = ConfigError::ConversionError {
        key: "db.timeout".to_string(),
        message: "Cannot convert to integer".to_string(),
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Type conversion failed"));
    assert!(error_msg.contains("Cannot convert to integer"));
    assert!(error_msg.contains("db.timeout"));
}

#[test]
fn test_index_out_of_bounds_error() {
    let error = ConfigError::IndexOutOfBounds { index: 5, len: 3 };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Index out of bounds"));
    assert!(error_msg.contains("5"));
    assert!(error_msg.contains("3"));
}

#[test]
fn test_substitution_error() {
    let error = ConfigError::SubstitutionError("Undefined variable: ${VAR}".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Variable substitution failed"));
    assert!(error_msg.contains("${VAR}"));
}

#[test]
fn test_substitution_depth_exceeded_error() {
    let error = ConfigError::SubstitutionDepthExceeded(64);
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("substitution depth exceeded"));
    assert!(error_msg.contains("64"));
}

#[test]
fn test_merge_error() {
    let error = ConfigError::MergeError("Type conflict".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Configuration merge failed"));
    assert!(error_msg.contains("Type conflict"));
}

#[test]
fn test_property_is_final_error() {
    let error = ConfigError::PropertyIsFinal("final.property".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("is final"));
    assert!(error_msg.contains("cannot be overridden"));
    assert!(error_msg.contains("final.property"));
}

#[test]
fn test_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let error: ConfigError = io_err.into();
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("IO error"));
}

#[test]
fn test_parse_error() {
    let error = ConfigError::ParseError("Invalid JSON format".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Parse error"));
    assert!(error_msg.contains("Invalid JSON format"));
}

#[test]
fn test_other_error() {
    let error = ConfigError::Other("Unknown error".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Configuration error"));
    assert!(error_msg.contains("Unknown error"));
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_from_value_error_no_value() {
    let value_err = ValueError::NoValue;
    let config_err: ConfigError = value_err.into();
    match config_err {
        ConfigError::PropertyHasNoValue(_) => {}
        _ => panic!("Expected PropertyHasNoValue error"),
    }
}

#[test]
fn test_from_value_error_type_mismatch() {
    let value_err = ValueError::TypeMismatch {
        expected: DataType::Bool,
        actual: DataType::Int32,
    };
    let config_err: ConfigError = value_err.into();
    match config_err {
        ConfigError::TypeMismatch {
            key,
            expected,
            actual,
        } => {
            assert_eq!(key, "");
            assert_eq!(expected, DataType::Bool);
            assert_eq!(actual, DataType::Int32);
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_from_value_error_conversion_failed() {
    let value_err = ValueError::ConversionFailed {
        from: DataType::String,
        to: DataType::Float64,
    };
    let config_err: ConfigError = value_err.into();
    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "");
            assert!(message.contains("From") && message.contains("to"));
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_from_value_error_conversion_error() {
    let value_err = ValueError::ConversionError("Custom error message".to_string());
    let config_err: ConfigError = value_err.into();
    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "");
            assert_eq!(message, "Custom error message");
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_from_value_error_index_out_of_bounds() {
    let value_err = ValueError::IndexOutOfBounds { index: 10, len: 5 };
    let config_err: ConfigError = value_err.into();
    match config_err {
        ConfigError::IndexOutOfBounds { index, len } => {
            assert_eq!(index, 10);
            assert_eq!(len, 5);
        }
        _ => panic!("Expected IndexOutOfBounds error"),
    }
}

#[test]
fn test_from_keyed_value_error_no_value() {
    let config_err = ConfigError::from(("server.port", ValueError::NoValue));

    match config_err {
        ConfigError::PropertyHasNoValue(key) => assert_eq!(key, "server.port"),
        _ => panic!("Expected PropertyHasNoValue error"),
    }
}

#[test]
fn test_from_keyed_value_error_type_mismatch() {
    let value_err = ValueError::TypeMismatch {
        expected: DataType::Int32,
        actual: DataType::String,
    };
    let config_err = ConfigError::from(("server.port", value_err));

    match config_err {
        ConfigError::TypeMismatch {
            key,
            expected,
            actual,
        } => {
            assert_eq!(key, "server.port");
            assert_eq!(expected, DataType::Int32);
            assert_eq!(actual, DataType::String);
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_from_keyed_value_error_conversion_failed() {
    let value_err = ValueError::ConversionFailed {
        from: DataType::String,
        to: DataType::Float64,
    };
    let config_err = ConfigError::from(("server.timeout", value_err));

    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "server.timeout");
            assert!(message.contains("From") && message.contains("to"));
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_from_keyed_value_error_conversion_error() {
    let value_err = ValueError::ConversionError("invalid value".to_string());
    let config_err = ConfigError::from(("server.timeout", value_err));

    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "server.timeout");
            assert_eq!(message, "invalid value");
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_from_keyed_value_error_index_out_of_bounds() {
    let value_err = ValueError::IndexOutOfBounds { index: 3, len: 1 };
    let config_err = ConfigError::from(("items", value_err));

    match config_err {
        ConfigError::IndexOutOfBounds { index, len } => {
            assert_eq!(index, 3);
            assert_eq!(len, 1);
        }
        _ => panic!("Expected IndexOutOfBounds error"),
    }
}

#[test]
fn test_from_keyed_value_error_json_serialization_error() {
    let value_err = ValueError::JsonSerializationError("unsupported".to_string());
    let config_err = ConfigError::from(("payload", value_err));

    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "payload");
            assert!(message.contains("JSON serialization error"));
            assert!(message.contains("unsupported"));
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_from_keyed_value_error_json_deserialization_error() {
    let value_err = ValueError::JsonDeserializationError("invalid json".to_string());
    let config_err = ConfigError::from(("payload", value_err));

    match config_err {
        ConfigError::ConversionError { key, message } => {
            assert_eq!(key, "payload");
            assert!(message.contains("JSON deserialization error"));
            assert!(message.contains("invalid json"));
        }
        _ => panic!("Expected ConversionError"),
    }
}

#[test]
fn test_get_conversion_or_type_error_carries_key() {
    let mut config = Config::new();
    config.set("my.key", "not-an-int").unwrap();

    let result: Result<i32, _> = config.get("my.key");

    assert!(matches!(
        result,
        Err(ConfigError::ConversionError { ref key, .. })
            | Err(ConfigError::TypeMismatch { ref key, .. }) if key == "my.key"
    ));
}

#[test]
fn test_get_type_mismatch_error_carries_key() {
    let mut config = Config::new();
    config.set("a.b", true).unwrap();

    let result: Result<i32, _> = config.get_strict("a.b");

    assert!(matches!(
        result,
        Err(ConfigError::TypeMismatch {
            ref key,
            expected: DataType::Int32,
            actual: DataType::Bool,
        }) if key == "a.b"
    ));
}

// ============================================================================
// Debug Trait Tests
// ============================================================================

#[test]
fn test_error_debug_format() {
    let error = ConfigError::PropertyNotFound("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("PropertyNotFound"));
}

// ============================================================================
// Error Type Matching Tests
// ============================================================================

#[test]
fn test_error_matching() {
    let errors = vec![
        ConfigError::PropertyNotFound("test".to_string()),
        ConfigError::PropertyHasNoValue("test".to_string()),
        ConfigError::TypeMismatch {
            key: "test.key".to_string(),
            expected: DataType::Int32,
            actual: DataType::String,
        },
        ConfigError::ConversionError {
            key: "test.key".to_string(),
            message: "test".to_string(),
        },
        ConfigError::IndexOutOfBounds { index: 1, len: 0 },
        ConfigError::SubstitutionError("test".to_string()),
        ConfigError::SubstitutionDepthExceeded(100),
        ConfigError::MergeError("test".to_string()),
        ConfigError::PropertyIsFinal("test".to_string()),
        ConfigError::ParseError("test".to_string()),
        ConfigError::Other("test".to_string()),
    ];

    for error in errors {
        // Verify that each error type can be properly formatted
        let msg = format!("{}", error);
        assert!(!msg.is_empty());
    }
}
