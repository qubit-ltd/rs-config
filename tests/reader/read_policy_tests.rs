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

    let options = ReadPolicy::builder()
        .boolean_policy(
            BooleanConversionPolicy::builder()
                .true_literal("enabled")
                .false_literal("disabled")
                .build()
                .expect("custom boolean literals should be distinct"),
        )
        .build();
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

    let options = ReadPolicy::builder()
        .blank_string_policy(BlankStringPolicy::TreatAsMissing)
        .build();
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
            ReadPolicy::builder_from(&ReadPolicy::env_friendly())
                .empty_item_policy(EmptyItemPolicy::Reject)
                .build(),
        )
        .set("PORTS", "8080,,8082")
        .expect("setting test config should succeed");

    let result = config.get::<Vec<u16>>("PORTS");

    assert!(matches!(result, Err(ConfigError::ConversionError { key, .. }) if key == "PORTS"));
}

#[test]
fn test_interpolation_sources_are_explicit_and_env_friendly_is_conversion_only() {
    let default_policy = ReadPolicy::default();
    let enabled_policy = ReadPolicy::builder_from(&default_policy)
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();

    assert_eq!(default_policy.interpolation_sources(), InterpolationSources::ConfigOnly);
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

    assert_eq!(options.interpolation_sources(), InterpolationSources::ConfigOnly);
    assert_eq!(options.conversion_policy(), ReadPolicy::default().conversion_policy());
    assert_eq!(options.max_interpolation_depth(), 64);
    assert_eq!(options.max_interpolation_expansions(), 4_096);
    assert_eq!(options.max_interpolation_output_bytes(), 1_048_576);
}

#[test]
fn test_interpolation_limit_builders_replace_defaults() {
    let options = ReadPolicy::builder()
        .max_interpolation_depth(8)
        .max_interpolation_expansions(16)
        .max_interpolation_output_bytes(32)
        .build();

    assert_eq!(options.max_interpolation_depth(), 8);
    assert_eq!(options.max_interpolation_expansions(), 16);
    assert_eq!(options.max_interpolation_output_bytes(), 32);
}

#[test]
fn test_read_policy_serde_uses_direct_interpolation_fields() {
    let options = ReadPolicy::builder()
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .max_interpolation_depth(8)
        .max_interpolation_expansions(16)
        .max_interpolation_output_bytes(32)
        .build();

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
    let string_options = StringConversionPolicy::builder().trim(true).build();
    let duration_options = DurationConversionPolicy::builder()
        .numeric_input_unit(DurationUnit::Milliseconds)
        .build();
    let options = ReadPolicy::builder()
        .string_policy(string_options.clone())
        .duration_policy(duration_options.clone())
        .build();

    assert_eq!(options.conversion_policy().string(), &string_options);
    assert_eq!(options.conversion_policy().duration(), &duration_options);
}

#[test]
fn test_collection_options_builder_is_exposed_directly() {
    let collection_options = CollectionConversionPolicy::builder().split_scalar_strings(true).build();
    let options = ReadPolicy::builder()
        .collection_policy(collection_options.clone())
        .build();

    assert_eq!(options.conversion_policy().collection(), &collection_options,);
}

#[test]
fn test_string_policy_preserves_its_own_blank_string_policy() {
    let string_policy = StringConversionPolicy::builder()
        .blank_string_policy(BlankStringPolicy::Reject)
        .build();

    let policy = ReadPolicy::builder()
        .blank_string_policy(BlankStringPolicy::TreatAsMissing)
        .string_policy(string_policy.clone())
        .build();

    assert_eq!(policy.conversion_policy().string(), &string_policy);
    assert_eq!(
        policy.conversion_policy().string().blank_string_policy(),
        BlankStringPolicy::Reject,
    );
}

#[test]
fn test_collection_policy_preserves_its_own_empty_item_policy() {
    let collection_policy = CollectionConversionPolicy::builder()
        .empty_item_policy(EmptyItemPolicy::Reject)
        .build();

    let policy = ReadPolicy::builder()
        .empty_item_policy(EmptyItemPolicy::Skip)
        .collection_policy(collection_policy.clone())
        .build();

    assert_eq!(policy.conversion_policy().collection(), &collection_policy,);
    assert_eq!(
        policy.conversion_policy().collection().empty_item_policy(),
        EmptyItemPolicy::Reject,
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
    let options = ReadPolicy::builder().conversion_limits(limits.clone()).build();
    let as_ref: &ConversionLimits = options.as_ref();
    assert_eq!(options.conversion_limits(), &limits);
    assert_eq!(as_ref, &limits);
}

#[test]
fn test_config_serialization_excludes_read_policy() {
    let mut config = Config::new();
    config
        .set_default_read_policy(
            ReadPolicy::builder_from(&ReadPolicy::env_friendly())
                .empty_item_policy(EmptyItemPolicy::Reject)
                .build(),
        )
        .set("PORTS", "8080,,8081")
        .expect("setting test config should succeed");

    let json = serde_json::to_string(&config).expect("serializing config should succeed");
    assert!(!json.contains("interpolation_sources"));
    let restored: Config = serde_json::from_str(&json).expect("deserializing config should succeed");

    assert_ne!(restored.default_read_policy(), config.default_read_policy());
    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
    assert!(restored.get::<Vec<u16>>("PORTS").is_err());
}

#[test]
fn test_read_policy_serde_defaults_are_readable() {
    let default_options: ReadPolicy = serde_json::from_str("{}").expect("empty options should use defaults");
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
        let options = ReadPolicy::builder().blank_string_policy(policy).build();
        let restored: ReadPolicy = serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(restored.conversion_policy().string().blank_string_policy(), policy);
    }

    for policy in [EmptyItemPolicy::Keep, EmptyItemPolicy::Skip, EmptyItemPolicy::Reject] {
        let options = ReadPolicy::builder().empty_item_policy(policy).build();
        let restored: ReadPolicy = serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(restored.conversion_policy().collection().empty_item_policy(), policy);
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
        let duration = DurationConversionPolicy::builder()
            .numeric_input_unit(unit)
            .suffixless_string_policy(SuffixlessDurationPolicy::Assume(unit))
            .output_unit(unit)
            .append_unit_suffix(false)
            .build();
        let options = ReadPolicy::builder().duration_policy(duration).build();
        let restored: ReadPolicy = serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
        assert_eq!(restored.conversion_policy().duration().numeric_input_unit(), unit,);
        assert_eq!(
            restored.conversion_policy().duration().suffixless_string_policy(),
            SuffixlessDurationPolicy::Assume(unit),
        );
        assert_eq!(restored.conversion_policy().duration().output_unit(), unit,);
        assert!(!restored.conversion_policy().duration().append_unit_suffix());
    }
}

/// Test serde round-trips the shared conversion options without a mirror DTO.
#[test]
fn test_read_policy_serde_round_trips_numeric_options() {
    let options = ReadPolicy::builder()
        .numeric_policy(NumericConversionPolicy::lossy())
        .numeric_limits(
            NumericConversionLimits::builder()
                .max_text_bytes(128)
                .max_big_integer_digits(64)
                .build(),
        )
        .build();
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
    let options = ReadPolicy::builder()
        .boolean_policy(
            BooleanConversionPolicy::builder()
                .true_literal("enabled")
                .false_literal("disabled")
                .case_sensitive(true)
                .build()
                .expect("custom boolean literals should be distinct"),
        )
        .build();
    let restored: ReadPolicy = serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();

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
