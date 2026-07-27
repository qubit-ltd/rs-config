// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for configurable read parsing behavior.

use qubit_config::{
    Config,
    ConfigError,
    field::ConfigField,
    options::ReadOptions,
};
use qubit_datatype::{
    BlankStringPolicy,
    BooleanConversionOptions,
    CollectionConversionOptions,
    DataConversionOptions,
    DurationConversionOptions,
    DurationUnit,
    EmptyItemPolicy,
    NumericConversionLimits,
    NumericConversionOptions,
    StringConversionOptions,
    SuffixlessDurationPolicy,
};

#[test]
fn test_global_env_friendly_options_parse_comma_separated_list() {
    let mut config = Config::new();
    config
        .set_read_options(ReadOptions::env_friendly())
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

    let options = ReadOptions::default().with_boolean_options(
        BooleanConversionOptions::strict()
            .with_true_literal("enabled")
            .expect("custom true literal should be distinct")
            .with_false_literal("disabled")
            .expect("custom false literal should be distinct"),
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

    let options = ReadOptions::default()
        .with_blank_string_policy(BlankStringPolicy::TreatAsMissing);
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
            ReadOptions::env_friendly()
                .with_empty_item_policy(EmptyItemPolicy::Reject),
        )
        .set("PORTS", "8080,,8082")
        .expect("setting test config should succeed");

    let result = config.get::<Vec<u16>>("PORTS");

    assert!(
        matches!(result, Err(ConfigError::ConversionError { key, .. }) if key == "PORTS")
    );
}

#[test]
fn test_environment_fallback_is_enabled_by_default_and_configurable() {
    let default_options = ReadOptions::default();
    let disabled_options = default_options
        .clone()
        .with_environment_fallback_enabled(false);

    assert!(default_options.is_environment_fallback_enabled());
    assert!(!disabled_options.is_environment_fallback_enabled());
    assert!(ReadOptions::env_friendly().is_environment_fallback_enabled());
    assert_eq!(default_options.max_interpolation_depth(), 64);
    assert_eq!(default_options.max_interpolation_expansions(), 4_096);
    assert_eq!(default_options.max_interpolation_output_bytes(), 1_048_576,);
}

#[test]
fn test_config_only_options_disable_environment_fallback() {
    let options = ReadOptions::config_only();

    assert!(!options.is_environment_fallback_enabled());
    assert_eq!(
        options.conversion_options(),
        ReadOptions::default().conversion_options()
    );
    assert_eq!(options.max_interpolation_depth(), 64);
    assert_eq!(options.max_interpolation_expansions(), 4_096);
    assert_eq!(options.max_interpolation_output_bytes(), 1_048_576);
}

#[test]
fn test_interpolation_limit_builders_replace_defaults() {
    let options = ReadOptions::default()
        .with_max_interpolation_depth(8)
        .with_max_interpolation_expansions(16)
        .with_max_interpolation_output_bytes(32);

    assert_eq!(options.max_interpolation_depth(), 8);
    assert_eq!(options.max_interpolation_expansions(), 16);
    assert_eq!(options.max_interpolation_output_bytes(), 32);
}

#[test]
fn test_read_options_serde_uses_direct_interpolation_fields() {
    let options = ReadOptions::default()
        .with_environment_fallback_enabled(false)
        .with_max_interpolation_depth(8)
        .with_max_interpolation_expansions(16)
        .with_max_interpolation_output_bytes(32);

    let json = serde_json::to_value(&options).expect("serialize read options");

    assert_eq!(json["environment_fallback_enabled"], false);
    assert_eq!(json["max_interpolation_depth"], 8);
    assert_eq!(json["max_interpolation_expansions"], 16);
    assert_eq!(json["max_interpolation_output_bytes"], 32);
    assert!(json.get("enabled").is_none());
    assert!(json.get("substitution").is_none());
    let restored: ReadOptions =
        serde_json::from_value(json).expect("deserialize read options");
    assert_eq!(restored, options);
}

#[test]
fn test_string_and_duration_options_are_delegated_to_conversion_options() {
    let string_options = StringConversionOptions::default().with_trim(true);
    let duration_options = DurationConversionOptions::default()
        .with_numeric_input_unit(DurationUnit::Milliseconds);
    let options = ReadOptions::default()
        .with_string_options(string_options.clone())
        .with_duration_options(duration_options.clone());

    assert_eq!(options.conversion_options().string(), &string_options);
    assert_eq!(options.conversion_options().duration(), &duration_options);
}

#[test]
fn test_collection_options_builder_is_exposed_directly() {
    let collection_options =
        CollectionConversionOptions::default().with_split_scalar_strings(true);
    let options = ReadOptions::default()
        .with_collection_options(collection_options.clone());

    assert_eq!(
        options.conversion_options().collection(),
        &collection_options,
    );
}

#[test]
fn test_from_and_as_ref_preserve_conversion_options() {
    let conversion = DataConversionOptions::env_friendly();

    let options = ReadOptions::from(conversion.clone());
    let as_ref: &DataConversionOptions = options.as_ref();

    assert_eq!(options.conversion_options(), &conversion);
    assert_eq!(as_ref, &conversion);
}

#[test]
fn test_config_serialization_preserves_read_options() {
    let mut config = Config::new();
    config
        .set_read_options(
            ReadOptions::env_friendly()
                .with_empty_item_policy(EmptyItemPolicy::Reject),
        )
        .set("PORTS", "8080,,8081")
        .expect("setting test config should succeed");

    let json = serde_json::to_string(&config)
        .expect("serializing config should succeed");
    let restored: Config = serde_json::from_str(&json)
        .expect("deserializing config should succeed");

    assert_eq!(restored.read_options(), config.read_options());
    assert!(restored.read_options().is_environment_fallback_enabled());
    assert!(matches!(
        restored.get::<Vec<u16>>("PORTS"),
        Err(ConfigError::ConversionError { key, .. }) if key == "PORTS"
    ));
}

#[test]
fn test_read_options_serde_defaults_are_readable() {
    let default_options: ReadOptions =
        serde_json::from_str("{}").expect("empty options should use defaults");
    let nested_defaults: ReadOptions =
        serde_json::from_value(serde_json::json!({
            "conversion": {
                "string": {},
                "boolean": {},
                "collection": {},
                "duration": {}
            }
        }))
        .expect("nested empty options should use defaults");
    let missing_boolean_defaults: ReadOptions =
        serde_json::from_value(serde_json::json!({
            "conversion": {
                "string": {},
                "collection": {},
                "duration": {}
            }
        }))
        .expect("omitted boolean options should use defaults");

    assert_eq!(default_options, ReadOptions::default());
    assert_eq!(nested_defaults, ReadOptions::default());
    assert_eq!(missing_boolean_defaults, ReadOptions::default());
}

#[test]
fn test_read_options_serde_round_trips_all_policy_variants() {
    for policy in [
        BlankStringPolicy::Preserve,
        BlankStringPolicy::TreatAsMissing,
        BlankStringPolicy::Reject,
    ] {
        let options = ReadOptions::default().with_blank_string_policy(policy);
        let restored: ReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap())
                .unwrap();
        assert_eq!(
            restored.conversion_options().string().blank_string_policy(),
            policy
        );
    }

    for policy in [
        EmptyItemPolicy::Keep,
        EmptyItemPolicy::Skip,
        EmptyItemPolicy::Reject,
    ] {
        let options = ReadOptions::default().with_empty_item_policy(policy);
        let restored: ReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap())
                .unwrap();
        assert_eq!(
            restored
                .conversion_options()
                .collection()
                .empty_item_policy(),
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
            .with_numeric_input_unit(unit)
            .with_suffixless_string_policy(SuffixlessDurationPolicy::Assume(
                unit,
            ))
            .with_output_unit(unit)
            .with_append_unit_suffix(false);
        let options = ReadOptions::default().with_duration_options(duration);
        let restored: ReadOptions =
            serde_json::from_str(&serde_json::to_string(&options).unwrap())
                .unwrap();
        assert_eq!(
            restored
                .conversion_options()
                .duration()
                .numeric_input_unit(),
            unit,
        );
        assert_eq!(
            restored
                .conversion_options()
                .duration()
                .suffixless_string_policy(),
            SuffixlessDurationPolicy::Assume(unit),
        );
        assert_eq!(
            restored.conversion_options().duration().output_unit(),
            unit,
        );
        assert!(
            !restored
                .conversion_options()
                .duration()
                .append_unit_suffix()
        );
    }
}

/// Test serde round-trips the shared conversion options without a mirror DTO.
#[test]
fn test_read_options_serde_round_trips_numeric_options() {
    let options = ReadOptions::default().with_numeric_options(
        NumericConversionOptions::lossy().with_limits(
            NumericConversionLimits::default()
                .with_max_text_bytes(128)
                .with_max_big_integer_digits(64),
        ),
    );
    let json = serde_json::to_value(&options).expect("serialize options");
    assert_eq!(
        json["conversion"]["numeric"]["fractional_to_integer"],
        "truncate",
    );
    assert_eq!(
        json["conversion"]["numeric"]["limits"]["max_text_bytes"],
        128,
    );

    let restored: ReadOptions =
        serde_json::from_value(json).expect("deserialize options");
    assert_eq!(restored, options);
}

#[test]
fn test_read_options_serde_boolean_literals_and_errors() {
    let options = ReadOptions::default().with_boolean_options(
        BooleanConversionOptions::strict()
            .with_true_literal("enabled")
            .expect("custom true literal should be distinct")
            .with_false_literal("disabled")
            .expect("custom false literal should be distinct")
            .with_case_sensitive(true)
            .expect("case sensitivity should preserve disjoint literals"),
    );
    let restored: ReadOptions =
        serde_json::from_str(&serde_json::to_string(&options).unwrap())
            .unwrap();

    assert!(
        restored
            .conversion_options()
            .boolean()
            .true_literals()
            .contains(&"enabled".to_string())
    );
    assert!(
        restored
            .conversion_options()
            .boolean()
            .false_literals()
            .contains(&"disabled".to_string())
    );
    assert!(restored.conversion_options().boolean().case_sensitive());

    let bad_true = serde_json::json!({
        "conversion": {
            "boolean": {
                "true_literals": ["yes"],
                "false_literals": ["yes"]
            }
        }
    });
    let bad_false = serde_json::json!({
        "conversion": {
            "boolean": {
                "true_literals": ["NO"],
                "false_literals": ["no"]
            }
        }
    });

    assert!(serde_json::from_value::<ReadOptions>(bad_true).is_err());
    assert!(serde_json::from_value::<ReadOptions>(bad_false).is_err());
}
