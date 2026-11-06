// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Distinguishes missing configuration values from conversion failures.

use qubit_config::{Config, ConfigError, ConfigErrorKind};
use qubit_datatype::DataType;

/// Returns whether an error represents a missing configuration value.
fn is_missing_value(error: &ConfigError) -> bool {
    match error.kind() {
        ConfigErrorKind::PropertyNotFound | ConfigErrorKind::PropertyHasNoValue => true,
        ConfigErrorKind::Value => error.value_missing().is_some(),
        _ => false,
    }
}

fn main() -> Result<(), ConfigError> {
    let mut config = Config::new();
    config.set_null("server.port", DataType::String)?;
    let error = config.get::<u16>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Value);
    assert_eq!(error.path(), Some("server.port"));
    assert!(error.value_missing().unwrap().is_unset());
    assert!(is_missing_value(&error));

    config.set("server.port", "not-a-port")?;
    let error = config.get::<u16>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert!(!is_missing_value(&error));
    Ok(())
}
