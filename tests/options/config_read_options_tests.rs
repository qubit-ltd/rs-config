/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for configurable read parsing behavior.

use qubit_config::{
    Config, ConfigError,
    field::ConfigField,
    options::{BlankStringPolicy, BooleanReadOptions, ConfigReadOptions, EmptyItemPolicy},
};

#[test]
fn test_global_env_friendly_options_parse_comma_separated_list() {
    let mut config = Config::new();
    config
        .set_read_options(ConfigReadOptions::env_friendly())
        .set("PORTS", "8080, 8081,,8082")
        .expect("setting test config should succeed");

    let ports = config
        .get::<Vec<u16>>("PORTS")
        .expect("comma-separated scalar string should parse as list");

    assert_eq!(ports, vec![8080, 8081, 8082]);
}

#[test]
fn test_field_options_can_add_custom_boolean_literals() {
    let mut config = Config::new();
    config
        .set("feature.flag", "enabled")
        .expect("setting test config should succeed");

    let options = ConfigReadOptions::default().with_boolean_options(
        BooleanReadOptions::strict()
            .with_true_literal("enabled")
            .with_false_literal("disabled"),
    );
    let enabled = config
        .read(
            ConfigField::<bool>::builder()
                .name("feature.flag")
                .read_options(options)
                .build(),
        )
        .expect("custom boolean literal should parse");

    assert!(enabled);
}

#[test]
fn test_field_options_can_treat_blank_string_as_missing() {
    let mut config = Config::new();
    config
        .set("primary.name", "   ")
        .expect("setting blank config should succeed");
    config
        .set("legacy.name", "fallback")
        .expect("setting fallback config should succeed");

    let options =
        ConfigReadOptions::default().with_blank_string_policy(BlankStringPolicy::TreatAsMissing);
    let name = config
        .read(
            ConfigField::<String>::builder()
                .name("primary.name")
                .alias("legacy.name")
                .read_options(options)
                .build(),
        )
        .expect("blank string should be skipped and alias should be read");

    assert_eq!(name, "fallback");
}

#[test]
fn test_collection_options_can_reject_empty_items() {
    let mut config = Config::new();
    config
        .set_read_options(
            ConfigReadOptions::env_friendly().with_empty_item_policy(EmptyItemPolicy::Reject),
        )
        .set("PORTS", "8080,,8082")
        .expect("setting test config should succeed");

    let result = config.get::<Vec<u16>>("PORTS");

    assert!(matches!(result, Err(ConfigError::ConversionError { key, .. }) if key == "PORTS"));
}
