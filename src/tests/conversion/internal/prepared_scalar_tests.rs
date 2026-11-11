// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;

use crate::ConfigError;
use crate::ConfigResult;
use crate::conversion::internal::PreparedScalar;

#[test]
fn test_successful_scalar_preparation_runs_once() {
    let scalar = PreparedScalar::new("${value}");
    let calls = Cell::new(0);
    let substitute = |_: &str| -> ConfigResult<String> {
        calls.set(calls.get() + 1);
        Ok("prepared".to_owned())
    };

    assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
    assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
    assert_eq!(calls.get(), 1);
}

#[test]
fn test_failed_scalar_preparation_is_not_cached() {
    let scalar = PreparedScalar::new("${value}");
    let calls = Cell::new(0);
    let substitute = |_: &str| -> ConfigResult<String> {
        calls.set(calls.get() + 1);
        if calls.get() == 1 {
            Err(ConfigError::SubstitutionError {
                path: "key".to_owned(),
                message: "unavailable".to_owned(),
            })
        } else {
            Ok("prepared".to_owned())
        }
    };

    assert!(scalar.text(true, &substitute).is_err());
    assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
    assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
    assert_eq!(calls.get(), 2);
}
