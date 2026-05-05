/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the field name builder state.

use qubit_config::{
    Config,
    field::ConfigField,
};

#[test]
fn test_config_field_name_builder_sets_primary_name_before_build() {
    let mut config = Config::new();
    config
        .set("server.port", 9090u16)
        .expect("setting config value should succeed");

    let field = ConfigField::<u16>::builder().name("server.port").build();
    let port = config.read(field).expect("named field should be readable");

    assert_eq!(port, 9090);
}

#[test]
fn test_config_field_name_builder_carries_pre_name_state() {
    let mut config = Config::new();
    config
        .set("PORT", "8080")
        .expect("setting alias value should succeed");

    let field = ConfigField::<u16>::builder()
        .name("server.port")
        .alias("PORT")
        .default(80)
        .build();
    let port = config.read(field).expect("alias should be readable");

    assert_eq!(port, 8080);
}
