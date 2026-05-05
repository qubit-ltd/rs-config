/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for default value adapters used by configuration reads.

use qubit_config::Config;

#[test]
fn test_into_config_default_accepts_scalar_default() {
    let config = Config::new();

    let value = config
        .get_or::<u16>("server.port", 8080u16)
        .expect("missing key should use scalar default");

    assert_eq!(value, 8080);
}

#[test]
fn test_into_config_default_accepts_string_slice_array_for_vec_string() {
    let config = Config::new();

    let paths = config
        .get_or::<Vec<String>>("app.paths", ["bin", "lib"])
        .expect("missing key should use string list default");

    assert_eq!(paths, vec!["bin".to_string(), "lib".to_string()]);
}
