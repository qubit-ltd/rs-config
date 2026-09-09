// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for default value adapters used by configuration reads.

use qubit_config::Config;

#[test]
fn test_value_default_is_only_converted_when_needed() {
    use std::cell::Cell;

    use qubit_value::IntoValueDefault;

    struct Counted<'a>(&'a Cell<usize>);
    impl IntoValueDefault<i32> for Counted<'_> {
        fn into_value_default(self) -> i32 {
            self.0.set(self.0.get() + 1);
            8080
        }
    }
    let mut config = Config::new();
    config.set("port", 9090_i32).unwrap();
    config.set("invalid", "not an integer").unwrap();
    let calls = Cell::new(0);
    assert_eq!(config.get_or::<i32>("port", Counted(&calls)).unwrap(), 9090);
    assert!(config.get_or::<i32>("invalid", Counted(&calls)).is_err());
    assert_eq!(calls.get(), 0);
    assert_eq!(config.get_or::<i32>("absent", Counted(&calls)).unwrap(), 8080);
    assert_eq!(calls.get(), 1);
}

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
