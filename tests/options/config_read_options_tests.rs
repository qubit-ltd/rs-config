/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for configurable read parsing behavior.

use qubit_config::{
    Config, ConfigError,
    field::ConfigField,
    options::{
        BlankStringPolicy, BooleanReadOptions, CollectionReadOptions, ConfigReadOptions,
        EmptyItemPolicy, StringReadOptions,
    },
};
use qubit_datatype::{
    BooleanConversionOptions, DataConversionOptions, DurationConversionOptions, DurationUnit,
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

#[test]
fn test_env_variable_substitution_option_is_explicit() {
    let default_options = ConfigReadOptions::default();
    let enabled_options = default_options
        .clone()
        .with_env_variable_substitution_enabled(true);
    let disabled_options = enabled_options
        .clone()
        .with_env_variable_substitution_enabled(false);

    assert!(!default_options.is_env_variable_substitution_enabled());
    assert!(enabled_options.is_env_variable_substitution_enabled());
    assert!(!disabled_options.is_env_variable_substitution_enabled());
    assert!(!ConfigReadOptions::env_friendly().is_env_variable_substitution_enabled());
}

#[test]
fn test_string_and_duration_options_are_delegated_to_conversion_options() {
    let string_options = StringReadOptions::default().with_trim(true);
    let duration_options =
        DurationConversionOptions::default().with_unit(DurationUnit::Milliseconds);
    let options = ConfigReadOptions::default()
        .with_string_options(string_options.clone())
        .with_duration_options(duration_options.clone());

    assert_eq!(options.conversion_options().string, string_options);
    assert_eq!(options.conversion_options().duration, duration_options);
}

#[test]
fn test_collection_options_builder_is_exposed_directly() {
    let collection_options = CollectionReadOptions::default().with_split_scalar_strings(true);
    let options = ConfigReadOptions::default().with_collection_options(collection_options.clone());

    assert_eq!(options.conversion_options().collection, collection_options);
}

#[test]
fn test_from_and_as_ref_preserve_conversion_options() {
    let conversion = DataConversionOptions::env_friendly();

    let options = ConfigReadOptions::from(conversion.clone());
    let as_ref: &DataConversionOptions = options.as_ref();

    assert_eq!(options.conversion_options(), &conversion);
    assert_eq!(as_ref, &conversion);
}

#[test]
fn test_config_serialization_preserves_read_options() {
    let mut config = Config::new();
    config
        .set_read_options(
            ConfigReadOptions::env_friendly()
                .with_empty_item_policy(EmptyItemPolicy::Reject)
                .with_env_variable_substitution_enabled(true),
        )
        .set("PORTS", "8080,,8081")
        .expect("setting test config should succeed");

    let json = serde_json::to_string(&config).expect("serializing config should succeed");
    let restored: Config =
        serde_json::from_str(&json).expect("deserializing config should succeed");

    assert_eq!(restored.read_options(), config.read_options());
    assert!(
        restored
            .read_options()
            .is_env_variable_substitution_enabled()
    );
    assert!(matches!(
        restored.get::<Vec<u16>>("PORTS"),
        Err(ConfigError::ConversionError { key, .. }) if key == "PORTS"
    ));
}

#[test]
fn test_config_read_options_serde_defaults_are_readable() {
    let default_options: ConfigReadOptions =
        serde_json::from_str("{}").expect("empty options should use defaults");
    let nested_defaults: ConfigReadOptions = serde_json::from_value(serde_json::json!({
        "conversion": {
            "string": {},
            "boolean": {},
            "collection": {},
            "duration": {}
        }
    }))
    .expect("nested empty options should use defaults");

    assert_eq!(default_options, ConfigReadOptions::default());
    assert_eq!(nested_defaults, ConfigReadOptions::default());
}

#[test]
fn test_config_read_options_serde_round_trips_all_policy_variants() {
    for policy in [
        BlankStringPolicy::Preserve,
        BlankStringPolicy::TreatAsMissing,
        BlankStringPolicy::Reject,
    ] {
        let options = ConfigReadOptions::default().with_blank_string_policy(policy);
        let restored: ConfigReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(
            restored.conversion_options().string.blank_string_policy,
            policy
        );
    }

    for policy in [
        EmptyItemPolicy::Keep,
        EmptyItemPolicy::Skip,
        EmptyItemPolicy::Reject,
    ] {
        let options = ConfigReadOptions::default().with_empty_item_policy(policy);
        let restored: ConfigReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(
            restored.conversion_options().collection.empty_item_policy,
            policy
        );
    }

    for unit in [
        DurationUnit::Nanoseconds,
        DurationUnit::Microseconds,
        DurationUnit::Milliseconds,
        DurationUnit::Seconds,
        DurationUnit::Minutes,
        DurationUnit::Hours,
        DurationUnit::Days,
    ] {
        let duration = DurationConversionOptions::default()
            .with_unit(unit)
            .with_append_unit_suffix(false);
        let options = ConfigReadOptions::default().with_duration_options(duration);
        let restored: ConfigReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(restored.conversion_options().duration.unit, unit);
        assert!(!restored.conversion_options().duration.append_unit_suffix);
    }
}

#[test]
fn test_config_read_options_serde_boolean_literals_and_errors() {
    let options = ConfigReadOptions::default().with_boolean_options(
        BooleanConversionOptions::strict()
            .with_true_literal("enabled")
            .with_false_literal("disabled")
            .with_case_sensitive(true),
    );
    let restored: ConfigReadOptions =
        serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();

    assert!(
        restored
            .conversion_options()
            .boolean
            .true_literals()
            .contains(&"enabled".to_string())
    );
    assert!(
        restored
            .conversion_options()
            .boolean
            .false_literals()
            .contains(&"disabled".to_string())
    );
    assert!(restored.conversion_options().boolean.case_sensitive);

    let bad_true = serde_json::json!({
        "conversion": {
            "boolean": {
                "true_literals": ["yes"],
                "false_literals": ["false", "0"]
            }
        }
    });
    let bad_false = serde_json::json!({
        "conversion": {
            "boolean": {
                "true_literals": ["true", "1"],
                "false_literals": ["no"]
            }
        }
    });

    assert!(serde_json::from_value::<ConfigReadOptions>(bad_true).is_err());
    assert!(serde_json::from_value::<ConfigReadOptions>(bad_false).is_err());
}

#[cfg(coverage)]
#[test]
fn test_coverage_touches_read_option_serde_defaults() {
    qubit_config::options::coverage_touch_config_read_option_serde_defaults();
}
