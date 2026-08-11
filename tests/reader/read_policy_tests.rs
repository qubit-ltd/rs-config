// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for configurable read parsing behavior.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::options::InterpolationSources;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::BooleanConversionPolicy;
use qubit_datatype::CollectionConversionPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionPolicy;
use qubit_datatype::DurationConversionPolicy;
use qubit_datatype::DurationUnit;
use qubit_datatype::EmptyItemPolicy;
use qubit_datatype::NumericConversionLimits;
use qubit_datatype::NumericConversionPolicy;
use qubit_datatype::StringConversionPolicy;
use qubit_datatype::SuffixlessDurationPolicy;

#[test]
fn test_global_env_friendly_options_parse_comma_separated_list() {
    let mut config = Config::new();
    config
        .set_default_read_policy(ReadPolicy::env_friendly())
        .set("PORTS", "8080, 8081,,8082")
        .expect("setting test config should succeed");

    let ports = config
        .get::<Vec<u16>>("PORTS")
        .expect("comma-separated scalar string should parse as list");

    assert_eq!(ports, vec![8080, 8081, 8082]);
}

#[test]
fn test_read_with_policy_can_add_custom_boolean_literals() {
    let mut config = Config::new();
    config
        .set("feature.flag", "enabled")
        .expect("setting test config should succeed");

    let options = ReadPolicy::default().with_boolean_policy(
        BooleanConversionPolicy::strict()
            .with_true_literal("enabled")
            .expect("custom true literal should be distinct")
            .with_false_literal("disabled")
            .expect("custom false literal should be distinct"),
    );
    let enabled = config
        .read_with(&options)
        .get::<bool>("feature.flag")
        .expect("custom boolean literal should parse");

    assert!(enabled);
}

#[test]
fn test_read_with_policy_can_treat_blank_string_as_missing() {
    let mut config = Config::new();
    config
        .set("primary.name", "   ")
        .expect("setting blank config should succeed");
    config
        .set("legacy.name", "fallback")
        .expect("setting fallback config should succeed");

    let options = ReadPolicy::default().with_blank_string_policy(BlankStringPolicy::TreatAsMissing);
    let name = config
        .read_with(&options)
        .get_any::<String>(["primary.name", "legacy.name"])
        .expect("blank string should be skipped and alias should be read");

    assert_eq!(name, "fallback");
}

#[test]
fn test_collection_options_can_reject_empty_items() {
    let mut config = Config::new();
    config
        .set_default_read_policy(
            ReadPolicy::env_friendly().with_empty_item_policy(EmptyItemPolicy::Reject),
        )
        .set("PORTS", "8080,,8082")
        .expect("setting test config should succeed");

    let result = config.get::<Vec<u16>>("PORTS");

    assert!(matches!(result, Err(ConfigError::ConversionError { key, .. }) if key == "PORTS"));
}

#[test]
fn test_interpolation_sources_are_explicit_and_env_friendly_is_conversion_only() {
    let default_policy = ReadPolicy::default();
    let enabled_policy = default_policy
        .clone()
        .with_interpolation_sources(InterpolationSources::ConfigThenEnv);

    assert_eq!(
        default_policy.interpolation_sources(),
        InterpolationSources::ConfigOnly
    );
    assert_eq!(
        enabled_policy.interpolation_sources(),
        InterpolationSources::ConfigThenEnv
    );
    assert_eq!(
        ReadPolicy::env_friendly().interpolation_sources(),
        InterpolationSources::ConfigOnly
    );
    assert_eq!(default_policy.max_interpolation_depth(), 64);
    assert_eq!(default_policy.max_interpolation_expansions(), 4_096);
    assert_eq!(default_policy.max_interpolation_output_bytes(), 1_048_576,);
}

#[test]
fn test_config_only_options_disable_environment_fallback() {
    let options = ReadPolicy::config_only();

    assert_eq!(
        options.interpolation_sources(),
        InterpolationSources::ConfigOnly
    );
    assert_eq!(
        options.conversion_policy(),
        ReadPolicy::default().conversion_policy()
    );
    assert_eq!(options.max_interpolation_depth(), 64);
    assert_eq!(options.max_interpolation_expansions(), 4_096);
    assert_eq!(options.max_interpolation_output_bytes(), 1_048_576);
}

#[test]
fn test_interpolation_limit_builders_replace_defaults() {
    let options = ReadPolicy::default()
        .with_max_interpolation_depth(8)
        .with_max_interpolation_expansions(16)
        .with_max_interpolation_output_bytes(32);

    assert_eq!(options.max_interpolation_depth(), 8);
    assert_eq!(options.max_interpolation_expansions(), 16);
    assert_eq!(options.max_interpolation_output_bytes(), 32);
}

#[test]
fn test_read_policy_serde_uses_direct_interpolation_fields() {
    let options = ReadPolicy::default()
        .with_interpolation_sources(InterpolationSources::ConfigThenEnv)
        .with_max_interpolation_depth(8)
        .with_max_interpolation_expansions(16)
        .with_max_interpolation_output_bytes(32);

    let json = serde_json::to_value(&options).expect("serialize read policy");

    assert_eq!(json["interpolation_sources"], "ConfigThenEnv");
    assert_eq!(json["max_interpolation_depth"], 8);
    assert_eq!(json["max_interpolation_expansions"], 16);
    assert_eq!(json["max_interpolation_output_bytes"], 32);
    assert!(json.get("enabled").is_none());
    assert!(json.get("substitution").is_none());
    let restored: ReadPolicy = serde_json::from_value(json).expect("deserialize read policy");
    assert_eq!(restored, options);
}

#[test]
fn test_string_and_duration_policies_are_delegated_to_conversion_policy() {
    let string_options = StringConversionPolicy::default().with_trim(true);
    let duration_options =
        DurationConversionPolicy::default().with_numeric_input_unit(DurationUnit::Milliseconds);
    let options = ReadPolicy::default()
        .with_string_policy(string_options.clone())
        .with_duration_policy(duration_options.clone());

    assert_eq!(options.conversion_policy().string(), &string_options);
    assert_eq!(options.conversion_policy().duration(), &duration_options);
}

#[test]
fn test_collection_options_builder_is_exposed_directly() {
    let collection_options = CollectionConversionPolicy::default().with_split_scalar_strings(true);
    let options = ReadPolicy::default().with_collection_policy(collection_options.clone());

    assert_eq!(
        options.conversion_policy().collection(),
        &collection_options,
    );
}

#[test]
fn test_from_and_as_ref_preserve_conversion_policy_and_limits() {
    let conversion = ConversionPolicy::env_friendly();

    let options = ReadPolicy::from(conversion.clone());
    let as_ref: &ConversionPolicy = options.as_ref();

    assert_eq!(options.conversion_policy(), &conversion);
    assert_eq!(as_ref, &conversion);

    let limits = ConversionLimits::default();
    let options = ReadPolicy::default().with_conversion_limits(limits.clone());
    let as_ref: &ConversionLimits = options.as_ref();
    assert_eq!(options.conversion_limits(), &limits);
    assert_eq!(as_ref, &limits);
}

#[test]
fn test_config_serialization_excludes_read_policy() {
    let mut config = Config::new();
    config
        .set_default_read_policy(
            ReadPolicy::env_friendly().with_empty_item_policy(EmptyItemPolicy::Reject),
        )
        .set("PORTS", "8080,,8081")
        .expect("setting test config should succeed");

    let json = serde_json::to_string(&config).expect("serializing config should succeed");
    assert!(!json.contains("interpolation_sources"));
    let restored: Config =
        serde_json::from_str(&json).expect("deserializing config should succeed");

    assert_ne!(restored.default_read_policy(), config.default_read_policy());
    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
    assert!(restored.get::<Vec<u16>>("PORTS").is_err());
}

#[test]
fn test_read_policy_serde_defaults_are_readable() {
    let default_options: ReadPolicy =
        serde_json::from_str("{}").expect("empty options should use defaults");
    let nested_defaults: ReadPolicy = serde_json::from_value(serde_json::json!({
        "conversion_policy": {
            "string": {},
            "boolean": {},
            "collection": {},
            "duration": {}
        }
    }))
    .expect("nested empty options should use defaults");
    let missing_boolean_defaults: ReadPolicy = serde_json::from_value(serde_json::json!({
        "conversion_policy": {
            "string": {},
            "collection": {},
            "duration": {}
        }
    }))
    .expect("omitted boolean options should use defaults");

    assert_eq!(default_options, ReadPolicy::default());
    assert_eq!(nested_defaults, ReadPolicy::default());
    assert_eq!(missing_boolean_defaults, ReadPolicy::default());
}

#[test]
fn test_read_policy_serde_round_trips_all_policy_variants() {
    for policy in [
        BlankStringPolicy::Preserve,
        BlankStringPolicy::TreatAsMissing,
        BlankStringPolicy::Reject,
    ] {
        let options = ReadPolicy::default().with_blank_string_policy(policy);
        let restored: ReadPolicy =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(
            restored.conversion_policy().string().blank_string_policy(),
            policy
        );
    }

    for policy in [
        EmptyItemPolicy::Keep,
        EmptyItemPolicy::Skip,
        EmptyItemPolicy::Reject,
    ] {
        let options = ReadPolicy::default().with_empty_item_policy(policy);
        let restored: ReadPolicy =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(
            restored
                .conversion_policy()
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
        let duration = DurationConversionPolicy::default()
            .with_numeric_input_unit(unit)
            .with_suffixless_string_policy(SuffixlessDurationPolicy::Assume(unit))
            .with_output_unit(unit)
            .with_append_unit_suffix(false);
        let options = ReadPolicy::default().with_duration_policy(duration);
        let restored: ReadPolicy =
            serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(
            restored.conversion_policy().duration().numeric_input_unit(),
            unit,
        );
        assert_eq!(
            restored
                .conversion_policy()
                .duration()
                .suffixless_string_policy(),
            SuffixlessDurationPolicy::Assume(unit),
        );
        assert_eq!(restored.conversion_policy().duration().output_unit(), unit,);
        assert!(!restored.conversion_policy().duration().append_unit_suffix());
    }
}

/// Test serde round-trips the shared conversion options without a mirror DTO.
#[test]
fn test_read_policy_serde_round_trips_numeric_options() {
    let options = ReadPolicy::default()
        .with_numeric_policy(NumericConversionPolicy::lossy())
        .with_numeric_limits(
            NumericConversionLimits::default()
                .with_max_text_bytes(128)
                .with_max_big_integer_digits(64),
        );
    let json = serde_json::to_value(&options).expect("serialize options");
    assert_eq!(
        json["conversion_policy"]["numeric"]["fractional_to_integer"],
        "truncate",
    );
    assert_eq!(json["conversion_limits"]["numeric"]["max_text_bytes"], 128,);

    let restored: ReadPolicy = serde_json::from_value(json).expect("deserialize options");
    assert_eq!(restored, options);
}

#[test]
fn test_read_policy_serde_boolean_literals_and_errors() {
    let options = ReadPolicy::default().with_boolean_policy(
        BooleanConversionPolicy::strict()
            .with_true_literal("enabled")
            .expect("custom true literal should be distinct")
            .with_false_literal("disabled")
            .expect("custom false literal should be distinct")
            .with_case_sensitive(true)
            .expect("case sensitivity should preserve disjoint literals"),
    );
    let restored: ReadPolicy =
        serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();

    assert!(
        restored
            .conversion_policy()
            .boolean()
            .true_literals()
            .contains(&"enabled".to_string())
    );
    assert!(
        restored
            .conversion_policy()
            .boolean()
            .false_literals()
            .contains(&"disabled".to_string())
    );
    assert!(restored.conversion_policy().boolean().case_sensitive());

    let bad_true = serde_json::json!({
        "conversion_policy": {
            "boolean": {
                "true_literals": ["yes"],
                "false_literals": ["yes"]
            }
        }
    });
    let bad_false = serde_json::json!({
        "conversion_policy": {
            "boolean": {
                "true_literals": ["NO"],
                "false_literals": ["no"]
            }
        }
    });

    assert!(serde_json::from_value::<ReadPolicy>(bad_true).is_err());
    assert!(serde_json::from_value::<ReadPolicy>(bad_false).is_err());
}
