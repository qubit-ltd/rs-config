// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;

use qubit_config::ConfigError;
use qubit_datatype::{DataConversionError, DataListConversionError, DataType, InvalidValueReason};
use qubit_value::{ValueAbsence, ValueError};

fn invalid_integer() -> DataConversionError {
    DataConversionError::invalid(
        DataType::String,
        DataType::Int32,
        InvalidValueReason::InvalidSyntax {
            expected: "a base-10 integer",
        },
    )
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
fn test_candidate_property_error_exposes_canonical_paths() {
    let error = ConfigError::PropertyCandidatesNotFound {
        paths: vec!["service.port".to_string(), "service.PORT".to_string()],
    };

    assert_eq!(
        error.kind(),
        qubit_config::ConfigErrorKind::PropertyNotFound
    );
    assert_eq!(error.path(), None);
    assert_eq!(
        error.candidate_paths(),
        Some(["service.port".to_string(), "service.PORT".to_string()].as_slice(),),
    );
    assert!(error.to_string().contains("service.port"));
    assert!(error.to_string().contains("service.PORT"));
}

#[test]
fn test_single_property_error_exposes_one_candidate_path() {
    let error = ConfigError::PropertyNotFound("service.port".to_string());

    assert_eq!(
        error.candidate_paths(),
        Some(["service.port".to_string()].as_slice()),
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
        DataConversionError::missing(DataType::String, DataType::Int32),
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
            source,
        } if key == "server.port"
            && matches!(
                source.reason(),
                Some(InvalidValueReason::InvalidSyntax { .. }),
            )
    ));
}

#[test]
fn test_value_error_requires_key_context() {
    let error = ConfigError::from((
        "server.port",
        ValueError::TypeMismatch {
            expected: DataType::Int32,
            actual: DataType::String,
        },
    ));
    assert!(matches!(
        error,
        ConfigError::TypeMismatch { key, .. } if key == "server.port"
    ));

    let error = ConfigError::from(("server.port", ValueError::DataConversion(invalid_integer())));
    assert!(matches!(
        error,
        ConfigError::ConversionError {
            key,
            source_index: None,
            ..
        } if key == "server.port"
    ));
}

#[test]
fn test_keyed_value_error_keeps_source_index() {
    let value_error =
        ValueError::DataListConversion(DataListConversionError::new(4, invalid_integer()));
    let error = ConfigError::from(("ports", value_error));
    assert!(matches!(
        error,
        ConfigError::ConversionError {
            key,
            source_index: Some(4),
            source,
        } if key == "ports"
            && source.kind()
                == qubit_datatype::DataConversionErrorKind::InvalidValue
    ));
}

#[test]
fn test_keyed_value_error_variants() {
    assert!(matches!(
        ConfigError::from((
            "empty",
            ValueError::NoValue(ValueAbsence::UnsetScalar {
                data_type: DataType::String,
            }),
        )),
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
        ConfigError::SubstitutionError {
            path: "service.url".to_string(),
            message: "missing".to_string(),
        }
        .to_string()
        .contains("missing")
    );
    assert!(
        ConfigError::SubstitutionDepthExceeded {
            path: "service.url".to_string(),
            max_depth: 16,
        }
        .to_string()
        .contains("16")
    );
    assert!(
        ConfigError::SubstitutionCycle {
            path: "service.url".to_string(),
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
