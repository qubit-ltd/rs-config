/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Focused tests for public configuration errors.

use qubit_config::ConfigError;
use qubit_datatype::{
    DataConversionError,
    DataType,
};

#[test]
fn test_config_error_maps_data_conversion_no_value_with_key() {
    let error =
        ConfigError::from_data_conversion_error("server.host", DataConversionError::NoValue);

    assert!(matches!(
        &error,
        ConfigError::PropertyHasNoValue(key) if key == "server.host"
    ));
    assert_eq!(error.to_string(), "Property 'server.host' has no value");
}

#[test]
fn test_config_error_maps_data_conversion_failure_with_key_context() {
    let error = ConfigError::from_data_conversion_error(
        "server.enabled",
        DataConversionError::ConversionFailed {
            from: DataType::String,
            to: DataType::Bool,
        },
    );

    assert!(matches!(
        &error,
        ConfigError::ConversionError { key, message }
            if key == "server.enabled" && message == "From string to bool"
    ));
    assert_eq!(
        error.to_string(),
        "Type conversion failed at 'server.enabled': From string to bool"
    );
}
