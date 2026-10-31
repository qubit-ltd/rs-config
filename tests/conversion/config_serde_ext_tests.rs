// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::HashMap;

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::UnknownFieldPolicy;
use qubit_config::conversion::ConfigSerdeExt;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use qubit_value::ValueError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[test]
fn deserialize_with_accepts_borrowed_prefix_and_explicit_options() {
    use qubit_config::ConfigDeserializeOptions;
    use qubit_config::UnknownFieldPolicy;
    #[derive(Debug, Deserialize, PartialEq)]
    struct One {
        port: u16,
    }
    let mut config = Config::new();
    config.set("server.port", "${default_port}").unwrap();
    config.set("server.extra", true).unwrap();
    config.set("default_port", 8080_u16).unwrap();
    let value: One = config
        .deserialize_with(
            std::borrow::Cow::Borrowed("server"),
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .unwrap();
    assert_eq!(value, One { port: 8080 });
    assert_eq!(
        ConfigDeserializeOptions::default().unknown_fields,
        UnknownFieldPolicy::Reject
    );
    assert!(!ConfigDeserializeOptions::default().interpolate);
}

#[test]
fn test_interpolation_expansions_accumulate_across_structured_fields() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(1).build())
        .build();
    config.set("x", "ok").unwrap();
    config.set("object.a", "${x}").unwrap();
    config.set("object.b", "${x}").unwrap();
    let error = config
        .deserialize_with::<std::collections::BTreeMap<String, String>>(
            "object",
            qubit_config::ConfigDeserializeOptions {
                interpolate: true,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(matches!(error,
        qubit_config::ConfigError::SubstitutionExpansionLimitExceeded {
            path, max_expansions: 1
        } if path == "object.b"));
}

#[test]
fn test_interpolation_budget_counts_fields_ignored_by_serde() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(1).build())
        .build();
    config.set("x", "ok").unwrap();
    config.set("object.label", "${x}").unwrap();
    config.set("object.z", "${x}").unwrap();

    let error = config
        .deserialize_with::<LabelSettings>(
            "object",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        ConfigError::SubstitutionExpansionLimitExceeded {
            path,
            max_expansions: 1
        } if path == "object.z"
    ));
}

#[derive(Debug, Deserialize, PartialEq)]
struct RetrySettings {
    max_attempts: u32,
    enabled: bool,
}

/// Both public entry points preserve the same default value and unknown path.
#[test]
fn deserialize_default_options_match_values_and_unknown_errors() {
    let mut config = Config::new();
    config.set("app.label", "${other}").unwrap();
    let plain: LabelSettings = config.deserialize("app").unwrap();
    let explicit: LabelSettings = config
        .deserialize_with("app", ConfigDeserializeOptions::default())
        .unwrap();
    assert_eq!(plain, explicit);
    assert_eq!(plain.label, "${other}");

    config.set("app.extra", true).unwrap();
    for error in [
        config.deserialize::<LabelSettings>("app").unwrap_err(),
        config
            .deserialize_with::<LabelSettings>("app", ConfigDeserializeOptions::default())
            .unwrap_err(),
    ] {
        assert!(matches!(error, ConfigError::UnknownProperties { paths } if paths == ["app.extra"]));
    }
}

/// Interpolation and unknown-field choices remain independent in scoped reads.
#[test]
fn deserialize_options_cover_all_combinations_and_empty_section_prefix() {
    for interpolate in [false, true] {
        for unknown_fields in [UnknownFieldPolicy::Reject, UnknownFieldPolicy::Ignore] {
            let options = ConfigDeserializeOptions {
                interpolate,
                unknown_fields,
            };
            let mut config = Config::new();
            config.set("app.label", "${label_source}").unwrap();
            config.set("label_source", "ready").unwrap();
            let expected = if interpolate { "ready" } else { "${label_source}" };
            let root: LabelSettings = config.deserialize_with("app", options).unwrap();
            let scoped: LabelSettings = config.section("app").unwrap().deserialize_with("", options).unwrap();
            assert_eq!(root.label, expected);
            assert_eq!(root, scoped);

            config.set("app.extra", true).unwrap();
            let result = config
                .section("app")
                .unwrap()
                .deserialize_with::<LabelSettings>("", options);
            match unknown_fields {
                UnknownFieldPolicy::Ignore => assert_eq!(result.unwrap().label, expected),
                UnknownFieldPolicy::Reject => assert!(matches!(
                    result.unwrap_err(),
                    ConfigError::UnknownProperties { paths } if paths == ["app.extra"]
                )),
            }
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct ServerSettings {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct EnvironmentSettings {
    enabled: bool,
    ports: Vec<u16>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct RawSettings {
    max_attempts: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct LabelSettings {
    label: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct OptionalLabelSettings {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct StrictRetrySettings {
    #[serde(alias = "attempts")]
    max_attempts: u32,
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
struct NestedStrictSettings {
    retry: StrictRetrySettings,
}

#[derive(Debug, Deserialize, PartialEq)]
struct OpenSettings {
    known: String,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ServerPayload {
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FlattenedSettings {
    #[serde(flatten)]
    server: ServerPayload,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum ServerChoice {
    Server { port: u16 },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ClosedServer {
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum ClosedServerChoice {
    Server(ClosedServer),
}

#[derive(Debug, Deserialize, PartialEq)]
struct StringPair {
    first: String,
    second: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ScopedInterpolationSettings {
    x: String,
    result: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct StringPairResult {
    result: String,
}

/// Dotted children bind interpolation to their selected subtree; exact JSON
/// object members remain values while lookup uses the calling reader's scope.
#[test]
fn structured_interpolation_distinguishes_subtree_and_exact_json_scope() {
    let options = ConfigDeserializeOptions {
        interpolate: true,
        ..Default::default()
    };

    let mut dotted = Config::new();
    dotted.set("x", "root").unwrap();
    dotted.set("retry.x", "parent").unwrap();
    dotted.set("retry.settings.x", "inner").unwrap();
    dotted.set("retry.settings.result", "${x}").unwrap();
    let subtree = dotted
        .section("retry")
        .unwrap()
        .deserialize_with::<ScopedInterpolationSettings>("settings", options)
        .unwrap();
    assert_eq!(subtree.x, "inner");
    assert_eq!(subtree.result, "inner");

    let mut exact = Config::new();
    exact.set("x", "root").unwrap();
    exact.set("retry.x", "parent").unwrap();
    exact
        .set("retry.settings", serde_json::json!({"x": "inner", "result": "${x}"}))
        .unwrap();
    let object = exact
        .section("retry")
        .unwrap()
        .deserialize_with::<ScopedInterpolationSettings>("settings", options)
        .unwrap();
    assert_eq!(object.x, "inner");
    assert_eq!(object.result, "parent");
}

/// Root fallback remains available, while disabled interpolation preserves
/// the original leaf text in both storage shapes.
#[test]
fn structured_interpolation_fallback_and_disabled_mode_match_both_shapes() {
    let mut dotted = Config::new();
    dotted.set("x", "root").unwrap();
    dotted.set("retry.settings.result", "${x}").unwrap();

    let mut exact = Config::new();
    exact.set("x", "root").unwrap();
    exact
        .set("retry.settings", serde_json::json!({"result": "${x}"}))
        .unwrap();

    for config in [&dotted, &exact] {
        let section = config.section("retry").unwrap();
        let interpolated: StringPairResult = section
            .deserialize_with(
                "settings",
                ConfigDeserializeOptions {
                    interpolate: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(interpolated.result, "root");

        let literal: StringPairResult = section
            .deserialize_with(
                "settings",
                ConfigDeserializeOptions {
                    interpolate: false,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(literal.result, "${x}");
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct ScalarStringSettings {
    enabled: String,
    attempts: String,
}

/// Deserializes retry settings from any supported configuration reader.
fn read_retry<R>(reader: &R) -> ConfigResult<RetrySettings>
where
    R: ConfigReader + ?Sized,
{
    reader.deserialize_with(
        "",
        ConfigDeserializeOptions {
            interpolate: true,
            unknown_fields: UnknownFieldPolicy::Reject,
        },
    )
}

#[test]
fn test_deserialize_interpolated_supports_generic_scoped_reader() {
    let mut config = Config::new();
    config
        .set("default_attempts", "4")
        .expect("default attempt count should be set");
    config
        .set("retry.max_attempts", "${default_attempts}")
        .expect("retry attempt placeholder should be set");
    config
        .set("retry.enabled", "true")
        .expect("retry enabled value should be set");

    let settings = read_retry(&config.section("retry").unwrap()).expect("scoped retry settings should deserialize");

    assert_eq!(
        settings,
        RetrySettings {
            max_attempts: 4,
            enabled: true,
        }
    );
}

/// Verifies the extension and inherent methods share one behavior.
#[test]
fn test_deserialize_matches_config_inherent_method() {
    let mut config = Config::new();
    config
        .set("server.host", "localhost")
        .expect("server host should be set");
    config.set("server.port", 8080_u16).expect("server port should be set");

    let inherent: ServerSettings = config
        .deserialize("server")
        .expect("inherent method should deserialize");
    let extension: ServerSettings =
        ConfigSerdeExt::deserialize(&config, "server").expect("extension method should deserialize");

    assert_eq!(extension, inherent);
}

/// Verifies ordinary structured reads preserve placeholders.
#[test]
fn test_deserialize_does_not_interpolate_scoped_values() {
    let mut config = Config::new();
    config
        .set("retry.max_attempts", "${default_attempts}")
        .expect("retry placeholder should be set");

    let settings: RawSettings = config
        .section("retry")
        .unwrap()
        .deserialize("")
        .expect("raw scoped settings should deserialize");

    assert_eq!(settings.max_attempts, "${default_attempts}");
}

/// Verifies a section read-options override controls structured conversion.
#[test]
fn test_deserialize_uses_section_read_policy_override() {
    let mut config = Config::new();
    config
        .set("service.enabled", "yes")
        .expect("service enabled value should be set");
    config
        .set("service.ports", "8080, 8081")
        .expect("service ports should be set");
    let options = ReadPolicy::env_friendly();
    let section = config.section("service").unwrap().read_with(&options);

    let settings: EnvironmentSettings = section
        .deserialize("")
        .expect("environment-style settings should deserialize");

    assert_eq!(
        settings,
        EnvironmentSettings {
            enabled: true,
            ports: vec![8080, 8081],
        }
    );
}

/// Verifies a selected subtree takes precedence over root fallback.
#[test]
fn test_deserialize_interpolated_prefers_selected_subtree() {
    let mut config = Config::new();
    config
        .set("attempts_value", "4")
        .expect("root attempt count should be set");
    config
        .set("retry.settings.attempts_value", "5")
        .expect("scoped attempt count should be set");
    config
        .set("retry.settings.max_attempts", "${attempts_value}")
        .expect("nested attempt placeholder should be set");
    config
        .set("retry.settings.enabled", true)
        .expect("nested enabled value should be set");

    let settings: RetrySettings = config
        .section("retry")
        .unwrap()
        .deserialize_with(
            "settings",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect("nested settings should deserialize");

    assert_eq!(settings.max_attempts, 5);
}

/// Verifies exact properties cannot coexist with visible descendants.
#[test]
fn test_deserialize_reports_root_relative_exact_subtree_conflict() {
    let mut config = Config::new();
    config
        .set("retry.policy", "fixed")
        .expect("exact retry policy should be set");
    config
        .set("retry.policy.mode", "strict")
        .expect("nested retry policy should be set");

    let error = config
        .section("retry")
        .unwrap()
        .deserialize::<serde_json::Value>("policy")
        .expect_err("exact property and descendants should conflict");

    assert!(matches!(
        error,
        ConfigError::KeyConflict { path, .. } if path == "retry.policy"
    ));
}

/// Verifies interpolation limits preserve their structured error and path.
#[test]
fn test_deserialize_interpolated_preserves_expansion_limit_error() {
    let mut config = Config::new();
    config
        .set("first", "4")
        .expect("first interpolation value should be set");
    config
        .set("second", "true")
        .expect("second interpolation value should be set");
    config
        .set("retry.label", "${first}-${second}")
        .expect("label placeholders should be set");
    let options = ReadPolicy::builder().max_interpolation_expansions(1).build();
    let section = config.section("retry").unwrap().read_with(&options);

    let error = section
        .deserialize_with::<LabelSettings>(
            "",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Reject,
            },
        )
        .expect_err("two placeholders should exceed the expansion limit");

    assert!(matches!(
        error,
        ConfigError::SubstitutionExpansionLimitExceeded {
            path,
            max_expansions: 1,
        } if path == "retry.label"
    ));
}

/// Verifies strict interpolated deserialization prepares a string once.
#[test]
fn test_deserialize_interpolated_strict_prepares_string_once() {
    let mut config = Config::new();
    config.set("name", "service").unwrap();
    config.set("settings.label", "${name}-api").unwrap();

    let settings: LabelSettings = config
        .deserialize_with(
            "settings",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Reject,
            },
        )
        .expect("strict interpolation should deserialize the declared field");

    assert_eq!(settings.label, "service-api");
}

/// Verifies lenient interpolated deserialization prepares a string once.
#[test]
fn test_deserialize_interpolated_lenient_prepares_string_once() {
    let mut config = Config::new();
    config.set("name", "service").unwrap();
    config.set("settings.label", "${name}-api").unwrap();
    config.set("settings.extra", "ignored").unwrap();

    let settings: LabelSettings = config
        .deserialize_with(
            "settings",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect("lenient interpolation should ignore the extra field");

    assert_eq!(settings.label, "service-api");
}

/// Verifies interpolated deserialization uses the root fallback once.
#[test]
fn test_deserialize_interpolated_uses_root_fallback_once() {
    let mut config = Config::new();
    config.set("suffix", "api").unwrap();
    config.set("settings.label", "service-${suffix}").unwrap();

    let settings: LabelSettings = config
        .section("settings")
        .unwrap()
        .deserialize_with(
            "",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Reject,
            },
        )
        .expect("the section should use the root fallback");

    assert_eq!(settings.label, "service-api");
}

/// Verifies an interpolated missing string remains absent.
#[test]
fn test_deserialize_interpolated_missing_string_stays_absent() {
    let mut config = Config::new();
    config.set("blank", "   ").unwrap();
    config.set("settings.label", "${blank}").unwrap();
    let policy = ReadPolicy::builder()
        .blank_string_policy(BlankStringPolicy::TreatAsMissing)
        .build();

    let settings: OptionalLabelSettings = config
        .read_with(&policy)
        .deserialize_with(
            "settings",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect("a prepared missing string should be omitted");

    assert_eq!(settings.label, None);
}

#[test]
fn test_deserialize_rejects_unknown_properties_with_root_relative_paths() {
    let mut config = Config::new();
    config.set("retry.max_attempts", 3_u32).unwrap();
    config.set("retry.max_attemps", 4_u32).unwrap();

    let error = config
        .deserialize::<StrictRetrySettings>("retry")
        .expect_err("strict deserialization should reject misspelled fields");

    assert!(matches!(
        error,
        ConfigError::UnknownProperties { paths }
            if paths == vec!["retry.max_attemps"]
    ));
}

#[test]
fn test_deserialize_lenient_explicitly_ignores_unknown_properties() {
    let mut config = Config::new();
    config.set("retry.max_attempts", 3_u32).unwrap();
    config.set("retry.extra", "ignored").unwrap();

    let settings = config
        .deserialize_with::<StrictRetrySettings>(
            "retry",
            ConfigDeserializeOptions {
                interpolate: false,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .expect("lenient deserialization should ignore extra fields");
    assert_eq!(
        settings,
        StrictRetrySettings {
            max_attempts: 3,
            enabled: false,
        }
    );
}

#[test]
fn test_deserialize_strict_supports_serde_alias_default_nested_map_and_flatten() {
    let mut config = Config::new();
    config.set("retry.attempts", 5_u32).unwrap();
    config.set("nested.retry.attempts", 5_u32).unwrap();
    config.set("labels.primary", "one").unwrap();
    config.set("open.known", "value").unwrap();
    config.set("open.extra", true).unwrap();

    let nested: NestedStrictSettings = config
        .deserialize("nested")
        .expect("alias and default should be honored");
    assert_eq!(nested.retry.max_attempts, 5);

    let dynamic: HashMap<String, String> = config
        .deserialize("labels")
        .expect("map keys should be dynamically consumed");
    assert_eq!(dynamic.get("primary"), Some(&"one".to_string()));

    let open: OpenSettings = config
        .deserialize("open")
        .expect("flatten should explicitly consume extra fields");
    assert_eq!(open.extra.get("extra"), Some(&serde_json::json!(true)));
}

fn assert_unknown_source_paths<T: DeserializeOwned>(input: Value, expected: &[&str]) {
    let mut config = Config::new();
    config.set("app.settings", input).unwrap();
    let section = config.section("app").unwrap();
    let errors = [
        config
            .deserialize::<T>("app.settings")
            .err()
            .expect("unknown fields must fail"),
        section
            .deserialize::<T>("settings")
            .err()
            .expect("scoped unknown fields must fail"),
    ];
    for error in errors {
        let actual: Vec<&str> = error
            .unknown_property_paths()
            .expect("error must carry unknown paths")
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(actual, expected);
        assert!(!error.to_string().contains("REVIEW_SECRET"));
    }
}

#[test]
fn test_unknown_source_paths_ignore_option_and_newtype_wrappers() {
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Payload {
        port: u16,
    }
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Wrapped(Payload);
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Outer {
        tls: Option<Wrapped>,
    }

    assert_unknown_source_paths::<Outer>(
        serde_json::json!({"tls": {"port": 443, "extra": "REVIEW_SECRET"}}),
        &["app.settings.tls.extra"],
    );
}

#[test]
fn test_unknown_source_paths_use_sequence_indices_and_enum_variants() {
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Payload {
        port: u16,
    }
    #[allow(dead_code)]
    #[derive(Deserialize)]
    enum Mode {
        Named { port: u16 },
    }
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Outer {
        items: Vec<Payload>,
        mode: Mode,
    }

    assert_unknown_source_paths::<Outer>(
        serde_json::json!({
            "items": [{"port": 80, "extra": "REVIEW_SECRET"}],
            "mode": {"Named": {"port": 443, "extra": "REVIEW_SECRET"}}
        }),
        &["app.settings.items[0].extra", "app.settings.mode.Named.extra"],
    );
}

#[test]
fn test_unknown_source_paths_preserve_real_punctuation_and_sort_final_paths() {
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Payload {
        port: u16,
    }

    assert_unknown_source_paths::<Payload>(
        serde_json::json!({"port": 80, ".extra": true, "?": true, "extra": true}),
        &["app.settings..extra", "app.settings.?", "app.settings.extra"],
    );
}

#[test]
fn test_unknown_source_paths_include_newtype_enum_variant() {
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Payload {
        port: u16,
    }
    #[allow(dead_code)]
    #[derive(Deserialize)]
    enum Mode {
        Named(Payload),
    }
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Outer {
        mode: Mode,
    }

    assert_unknown_source_paths::<Outer>(
        serde_json::json!({"mode": {"Named": {"port": 443, "extra": "REVIEW_SECRET"}}}),
        &["app.settings.mode.Named.extra"],
    );
}

#[test]
fn test_unknown_source_paths_support_empty_root_and_deduplicate_final_paths() {
    #[derive(Debug, Deserialize)]
    struct Empty {}
    #[derive(Debug, Deserialize)]
    struct Inner {}
    #[derive(Debug, Deserialize)]
    struct Outer {
        #[allow(dead_code)]
        a: Inner,
    }

    let mut root_config = Config::new();
    root_config.set("extra", true).unwrap();
    let root_error = root_config
        .deserialize::<Empty>("")
        .expect_err("unknown root fields must fail");
    assert_eq!(
        root_error.unknown_property_paths(),
        Some(["extra".to_owned()].as_slice())
    );

    let mut config = Config::new();
    config
        .set("settings", serde_json::json!({"a.b": true, "a": {"b": true}}))
        .unwrap();
    let error = config
        .deserialize::<Outer>("settings")
        .expect_err("unknown fields with the same displayed path must fail");
    assert_eq!(
        error.unknown_property_paths(),
        Some(["settings.a.b".to_owned()].as_slice())
    );
}

#[test]
fn test_unknown_source_paths_do_not_replace_conversion_errors() {
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct Payload {
        port: u16,
    }

    let mut config = Config::new();
    config
        .set("settings", serde_json::json!({"port": "invalid", "extra": true}))
        .unwrap();
    let error = config
        .deserialize::<Payload>("settings")
        .expect_err("a conversion error must take precedence over unknown fields");
    assert!(!matches!(error, ConfigError::UnknownProperties { .. }));
    assert_eq!(error.path(), Some("settings.port"));
}

#[test]
fn test_reject_only_reports_unknown_fields_observable_to_source_callback() {
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.typo", true).unwrap();

    let error = config
        .deserialize::<ServerPayload>("server")
        .expect_err("an ordinary struct exposes the unknown field");
    assert!(matches!(error, ConfigError::UnknownProperties { paths } if paths == ["server.typo"]));

    let flattened: FlattenedSettings = config
        .deserialize("server")
        .expect("flatten buffering can hide an ignored field from the source callback");
    assert_eq!(flattened.server.port, 8080);

    let choice: ServerChoice = config
        .deserialize("server")
        .expect("untagged buffering can hide an ignored field from the source callback");
    assert!(matches!(choice, ServerChoice::Server { port: 8080 }));
}

#[test]
fn test_closed_struct_targets_reject_unknown_fields() {
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.typo", "SYNTHETIC_REVIEW_SECRET").unwrap();

    for error in [
        config.deserialize::<ClosedServer>("server").unwrap_err(),
        config.deserialize::<ClosedServerChoice>("server").unwrap_err(),
    ] {
        assert!(matches!(&error, ConfigError::DeserializeError { .. }));
        assert!(!error.to_string().contains("SYNTHETIC_REVIEW_SECRET"));
    }
}

#[test]
fn test_ignore_still_rejects_unadmitted_input() {
    let mut config = Config::new();
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.extra", "x".repeat(128)).unwrap();
    config.set_default_read_policy(
        ReadPolicy::builder()
            .conversion_limits(
                ConversionLimits::builder()
                    .operation_limits(
                        ConversionOperationLimits::builder()
                            .max_structured_payload_bytes(10)
                            .build(),
                    )
                    .build(),
            )
            .build(),
    );

    let error = config
        .deserialize_with::<ServerSettings>(
            "server",
            ConfigDeserializeOptions {
                interpolate: false,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        )
        .unwrap_err();
    assert!(!matches!(&error, ConfigError::UnknownProperties { .. }));
    assert!(error.to_string().contains("StructuredPayloadBytes"));
}

#[test]
fn test_deserialize_unknown_properties_are_sorted_and_deduplicated() {
    let mut config = Config::new();
    config.set("retry.max_attempts", 3_u32).unwrap();
    config.set("retry.zed", true).unwrap();
    config.set("retry.aaa", true).unwrap();

    let error = config
        .deserialize::<StrictRetrySettings>("retry")
        .expect_err("unknown fields should be reported");
    assert_eq!(
        error.unknown_property_paths(),
        Some(["retry.aaa".to_string(), "retry.zed".to_string()].as_slice())
    );
}

#[test]
fn test_serde_materialization_shares_one_conversion_operation_limit() {
    for maximum in [3_u64, 4] {
        let limits = ConversionLimits::builder()
            .operation_limits(ConversionOperationLimits::builder().max_output_bytes(maximum).build())
            .build();
        let mut config = Config::new();
        config.set_default_read_policy(ReadPolicy::builder().conversion_limits(limits).build());
        config.set("first", "aa").expect("first value should be stored");
        config.set("second", "bb").expect("second value should be stored");

        let result = config.deserialize::<StringPair>("");
        if maximum == 3 {
            let error = result.expect_err("second field exhausts shared output bytes");
            assert_eq!(error.kind(), ConfigErrorKind::Conversion);
            assert_eq!(error.path(), Some("second"));
            assert!(error.to_string().contains("OutputBytes"));
        } else {
            let pair = result.expect("four output bytes should fit");
            assert_eq!(pair.first, "aa");
            assert_eq!(pair.second, "bb");
        }
    }
}

#[test]
fn test_scalar_string_sequence_charges_source_and_items_once() {
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_items(2)
                .max_input_bytes(10)
                .max_output_bytes(0)
                .build(),
        )
        .build();
    let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .conversion_limits(limits)
        .build();
    let mut config = Config::new();
    config
        .set_default_read_policy(policy.clone())
        .set("ports", "8080, 8081")
        .expect("ports should be stored");

    let ports: Vec<u16> = config
        .read_with(&policy)
        .deserialize("ports")
        .expect("a scalar list should fit exactly within input and item limits");

    assert_eq!(ports, vec![8080, 8081]);
}

#[test]
fn test_scalar_string_sequence_preserves_admitted_item_errors() {
    let policy = ReadPolicy::env_friendly();
    let mut config = Config::new();
    config
        .set_default_read_policy(policy.clone())
        .set("ports", "8080, invalid")
        .expect("ports should be stored");

    let error = config
        .read_with(&policy)
        .deserialize::<Vec<u16>>("ports")
        .expect_err("an invalid admitted item must fail conversion");

    assert!(error.to_string().contains("ports[1]"));
}

#[test]
fn test_scalar_boolean_and_number_to_string_charge_output() {
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_items(2)
                .max_output_bytes(0)
                .build(),
        )
        .build();
    let policy = ReadPolicy::builder().conversion_limits(limits).build();
    let mut config = Config::new();
    config.set_default_read_policy(policy.clone());
    config.set("enabled", true).expect("enabled should be stored");
    config.set("attempts", 3_u16).expect("attempts should be stored");

    let error = config
        .read_with(&policy)
        .deserialize::<ScalarStringSettings>("")
        .expect_err("scalar boolean and number conversions must charge output bytes");

    assert!(error.to_string().contains("OutputBytes"));
}

/// Intermediate JSON capacity does not grant final String output capacity.
#[test]
fn test_intermediate_projection_preserves_final_output_limit() {
    let limits = ConversionLimits::builder()
        .operation_limits(ConversionOperationLimits::builder().max_output_bytes(0).build())
        .build();
    let policy = ReadPolicy::builder().conversion_limits(limits).build();
    let mut config = Config::new();
    config.set("name", "worker").expect("store source");
    let error = config
        .read_with(&policy)
        .deserialize::<String>("name")
        .expect_err("final String output remains forbidden");
    assert!(error.to_string().contains("OutputBytes"));
}

/// The intermediate projection is still bounded before typed conversion.
#[test]
fn test_intermediate_projection_preserves_structured_payload_limit() {
    let limits = ConversionLimits::builder()
        .operation_limits(
            ConversionOperationLimits::builder()
                .max_output_bytes(0)
                .max_structured_payload_bytes(2)
                .build(),
        )
        .build();
    let policy = ReadPolicy::builder().conversion_limits(limits).build();
    let mut config = Config::new();
    config.set("port", "8080").expect("store source");
    let error = config
        .read_with(&policy)
        .deserialize::<u16>("port")
        .expect_err("intermediate source exceeds structured payload bound");
    assert!(matches!(
        error,
        ConfigError::ValueError {
            source: ValueError::JsonProjectionLimit { .. },
            ..
        }
    ));
}
