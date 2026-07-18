// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for stable configuration error classification and context.

use qubit_config::{
    ConfigError,
    ConfigErrorKind,
};
use qubit_datatype::{
    DataConversionError,
    DataType,
    InvalidValueReason,
};

#[test]
fn test_config_error_exposes_stable_kind_and_path() {
    let error = ConfigError::from_data_conversion_error(
        "server.timeout",
        DataConversionError::invalid(
            DataType::String,
            DataType::Duration,
            InvalidValueReason::InvalidSyntax {
                expected: "a duration",
            },
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
