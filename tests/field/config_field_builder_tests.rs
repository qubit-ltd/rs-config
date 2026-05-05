/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for field builder read declarations.

use qubit_config::{
    Config,
    field::ConfigField,
    options::ConfigReadOptions,
};

#[test]
fn test_config_field_builder_applies_alias_default_and_options() {
    let mut config = Config::new();
    config
        .set("FEATURE_ENABLED", "yes")
        .expect("setting alias value should succeed");

    let enabled = config
        .read(
            ConfigField::<bool>::builder()
                .name("feature.enabled")
                .alias("FEATURE_ENABLED")
                .default(false)
                .read_options(ConfigReadOptions::env_friendly())
                .build(),
        )
        .expect("field-level options should parse env-friendly boolean");
    let defaulted = config
        .read(
            ConfigField::<u16>::builder()
                .name("server.port")
                .default(8080)
                .build(),
        )
        .expect("missing field should use default");

    assert!(enabled);
    assert_eq!(defaulted, 8080);
}

#[test]
fn test_config_field_builder_preserves_alias_priority() {
    let mut config = Config::new();
    config
        .set("PRIMARY_ALIAS", "first")
        .expect("setting first alias should succeed");
    config
        .set("SECONDARY_ALIAS", "second")
        .expect("setting second alias should succeed");

    let value = config
        .read(
            ConfigField::<String>::builder()
                .name("service.name")
                .alias("PRIMARY_ALIAS")
                .alias("SECONDARY_ALIAS")
                .build(),
        )
        .expect("first configured alias should be read");

    assert_eq!(value, "first");
}
