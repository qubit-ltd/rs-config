// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// Tests for explicit configuration interpolation.

use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::UnknownFieldPolicy;
use qubit_config::options::InterpolationSources;
use qubit_config::options::ReadPolicy;
use qubit_datatype::BlankStringPolicy;
use qubit_datatype::CollectionConversionPolicy;
use qubit_value::ValueError;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Endpoint {
    url: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct RawEndpoint {
    url: String,
}

#[test]
fn test_get_preserves_placeholder() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("url", "http://${host}/api").expect("set URL");

    let value = config.get::<String>("url").expect("read URL");

    assert_eq!(value, "http://${host}/api");
}

#[test]
fn test_get_optional_and_get_any_preserve_placeholders() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("URL", "http://${host}/api").expect("set URL");

    let optional = config.get_optional::<String>("URL").expect("read optional URL");
    let any = config.get_any::<String>(["url", "URL"]).expect("read aliased URL");

    assert_eq!(optional.as_deref(), Some("http://${host}/api"));
    assert_eq!(any, "http://${host}/api");
}

#[test]
fn test_get_or_uses_default_only_when_value_is_missing() {
    let mut config = Config::new();
    config.set("port", "invalid").expect("set port");

    let missing = config
        .get_or::<u16>("missing", 8080)
        .expect("missing port should use default");
    let invalid = config.get_or::<u16>("port", 8080);

    assert_eq!(missing, 8080);
    assert!(matches!(
        invalid,
        Err(ConfigError::ConversionError { key, .. }) if key == "port"
    ));
}

#[test]
fn test_get_interpolated_resolves_string_and_numeric_values() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("port", "8080").expect("set port");
    config.set("url", "http://${host}:${port}/api").expect("set URL");
    config.set("server.port", "${port}").expect("set server port");

    let url = config.get_interpolated::<String>("url").expect("interpolate URL");
    let port = config
        .get_interpolated::<u16>("server.port")
        .expect("interpolate numeric port");

    assert_eq!(url, "http://localhost:8080/api");
    assert_eq!(port, 8080);
}

#[test]
fn test_interpolation_expansions_accumulate_across_list_elements() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(1).build())
        .build();
    config.set("x", "ok").unwrap();
    config.set("items", ["${x}", "${x}"]).unwrap();
    let error = config.get_interpolated::<Vec<String>>("items").unwrap_err();
    assert_eq!(
        error.kind(),
        qubit_config::ConfigErrorKind::SubstitutionExpansionLimitExceeded
    );
    assert_eq!(error.path(), Some("items"));
    assert_eq!(error.source_index(), Some(1));
    assert!(matches!(
        error,
        ConfigError::IndexedSubstitutionError { source_index: 1, source }
            if matches!(source.as_ref(), ConfigError::SubstitutionExpansionLimitExceeded {
                path,
                max_expansions: 1,
            } if path == "items")
    ));
}

#[test]
fn test_interpolated_collection_reports_earlier_conversion_error_first() {
    let mut config = Config::new();
    config.set("items", vec!["bad", "${missing}"]).unwrap();

    let error = config.get_interpolated::<Vec<u16>>("items").unwrap_err();

    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("items"));
    assert_eq!(error.source_index(), Some(0));
}

#[test]
fn test_interpolated_collection_substitution_error_keeps_original_index() {
    let mut config = Config::new();
    config.set("items", vec!["valid", "${missing}"]).unwrap();

    let error = config.get_interpolated::<Vec<String>>("items").unwrap_err();

    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Substitution);
    assert_eq!(error.path(), Some("items"));
    assert_eq!(error.source_index(), Some(1));
}

#[test]
fn test_interpolated_string_collection_preserves_collection_and_split_semantics() {
    let policy = ReadPolicy::builder()
        .collection_policy(CollectionConversionPolicy::builder().split_scalar_strings(true).build())
        .build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("empty", Vec::<String>::new()).unwrap();
    config.set("items", vec!["1,2"]).unwrap();
    config.set("scalar", "1,2").unwrap();

    assert!(config.get_interpolated::<Vec<String>>("empty").unwrap().is_empty());
    let collection_error = config.get_interpolated::<Vec<u16>>("items").unwrap_err();
    assert_eq!(collection_error.source_index(), Some(0));
    assert_eq!(config.get_interpolated::<Vec<String>>("scalar").unwrap(), ["1", "2"]);
}

#[test]
fn test_interpolated_collection_conversion_budgets_accumulate_across_items() {
    fn read_with_limits(max_input_bytes: u64, max_output_bytes: u64) -> Result<Vec<String>, ConfigError> {
        let limits = qubit_datatype::ConversionLimits::builder()
            .operation_limits(
                qubit_datatype::ConversionOperationLimits::builder()
                    .max_items(2)
                    .max_input_bytes(max_input_bytes)
                    .max_output_bytes(max_output_bytes)
                    .build(),
            )
            .build();
        let policy = ReadPolicy::builder().conversion_limits(limits).build();
        let mut config = Config::builder().default_read_policy(policy).build();
        config.set("items", vec!["1", "2"]).unwrap();
        config.get_interpolated("items")
    }

    assert_eq!(read_with_limits(2, 2).unwrap(), ["1", "2"]);
    for (input_limit, output_limit) in [(1, 2), (2, 1)] {
        let error = read_with_limits(input_limit, output_limit).unwrap_err();
        assert_eq!(error.source_index(), Some(1));
    }

    let operation = qubit_datatype::ConversionOperationLimits::builder()
        .max_items(0)
        .build();
    let policy = ReadPolicy::builder()
        .conversion_limits(
            qubit_datatype::ConversionLimits::builder()
                .operation_limits(operation)
                .build(),
        )
        .build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("items", vec!["1"]).unwrap();
    let error = config.get_interpolated::<Vec<String>>("items").unwrap_err();
    assert_eq!(error.source_index(), Some(0));
}

#[test]
fn test_interpolated_collection_policy_missing_item_keeps_index_and_path() {
    let policy = ReadPolicy::builder()
        .blank_string_policy(BlankStringPolicy::TreatAsMissing)
        .build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("service.items", vec!["1", ""]).unwrap();
    let service = config.section("service").unwrap();

    let error = service.get_interpolated::<Vec<u16>>("items").unwrap_err();
    assert_eq!(error.path(), Some("service.items"));
    assert_eq!(error.source_index(), Some(1));

    for error in [
        service.get_optional_interpolated::<Vec<u16>>("items").unwrap_err(),
        service.get_interpolated_or::<Vec<u16>>("items", vec![9]).unwrap_err(),
        service
            .get_any_interpolated::<Vec<u16>>(["items", "fallback"])
            .unwrap_err(),
    ] {
        assert_eq!(error.path(), Some("service.items"));
        assert_eq!(error.source_index(), Some(1));
    }
}

#[test]
fn test_interpolation_precheck_and_conversion_share_scalar_expansion() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(1).build())
        .build();
    config.set("x", "ok").unwrap();
    config.set("value", "${x}").unwrap();

    assert_eq!(config.get_interpolated::<String>("value").unwrap(), "ok");
}

#[test]
fn test_interpolation_budget_is_fresh_for_independent_reads() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(1).build())
        .build();
    config.set("x", "ok").unwrap();
    config.set("value", "${x}").unwrap();

    assert_eq!(config.get_interpolated::<String>("value").unwrap(), "ok");
    assert_eq!(config.get_interpolated::<String>("value").unwrap(), "ok");
}

#[test]
fn test_interpolation_budget_accumulates_across_missing_candidates() {
    let mut limited = Config::builder()
        .default_read_policy(
            ReadPolicy::builder()
                .max_interpolation_expansions(1)
                .blank_string_policy(BlankStringPolicy::TreatAsMissing)
                .build(),
        )
        .build();
    limited.set("blank", "").unwrap();
    limited.set("x", "ok").unwrap();
    limited.set("first", "${blank}").unwrap();
    limited.set("second", "${x}").unwrap();
    assert_eq!(
        limited
            .read_policy()
            .conversion_policy()
            .string()
            .normalize_optional(""),
        Ok(None)
    );

    let error = limited
        .get_optional_any_interpolated::<String>(["first", "second"])
        .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::SubstitutionExpansionLimitExceeded {
            path,
            max_expansions: 1
        } if path == "second"
    ));

    let mut sufficient = Config::builder()
        .default_read_policy(
            ReadPolicy::builder()
                .max_interpolation_expansions(2)
                .blank_string_policy(BlankStringPolicy::TreatAsMissing)
                .build(),
        )
        .build();
    sufficient.set("blank", "").unwrap();
    sufficient.set("x", "ok").unwrap();
    sufficient.set("first", "${blank}").unwrap();
    sufficient.set("second", "${x}").unwrap();
    assert_eq!(
        sufficient
            .get_optional_any_interpolated::<String>(["first", "second"])
            .unwrap()
            .as_deref(),
        Some("ok")
    );
}

/// Verifies interpolation preserves empty placeholders while resolving later
/// valid placeholders in the same string.
#[test]
fn test_interpolation_skips_empty_placeholder_and_resolves_following_value() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config
        .set("url", "${}-${host}")
        .expect("set URL with empty placeholder");

    let value = config.get_interpolated::<String>("url").expect("interpolate URL");

    assert_eq!(value, "${}-localhost");
}

/// Verifies scoped interpolated reads fall back to root configuration keys.
#[test]
fn test_section_get_interpolated_falls_back_to_root_configuration() {
    let mut config = Config::new();
    config
        .set("global.host", "api.example.test")
        .expect("root host should be set");
    config
        .set("service.url", "https://${global.host}/v1")
        .expect("scoped URL should be set");

    let service = config.section("service").expect("service section should be valid");
    let url = service
        .get_interpolated::<String>("url")
        .expect("scoped interpolation should fall back to root keys");

    assert_eq!(url, "https://api.example.test/v1");
}

#[test]
fn test_section_interpolated_collection_first_item_falls_back_to_root() {
    let mut config = Config::new();
    config.set("global.port", "8080").unwrap();
    config
        .set("service.ports", vec!["${global.port}", "${missing}"])
        .unwrap();
    let service = config.section("service").unwrap();

    assert_eq!(service.get_interpolated::<u16>("ports").unwrap(), 8080);
}

#[test]
fn test_interpolated_collection_first_item_obeys_optional_default_and_candidate_reads() {
    let mut config = Config::new();
    config.set("base", "8080").unwrap();
    config.set("primary", vec!["${base}", "${missing}"]).unwrap();
    config.set("ALIAS", vec!["${base}", "${missing}"]).unwrap();

    assert_eq!(config.get_optional_interpolated::<u16>("primary").unwrap(), Some(8080));
    assert_eq!(config.get_interpolated_or::<u16>("primary", 9).unwrap(), 8080);
    assert_eq!(config.get_any_interpolated::<u16>(["alias", "ALIAS"]).unwrap(), 8080);
}

#[test]
fn test_interpolated_collection_first_item_uses_expansion_budget_and_output_limit() {
    let mut limited = Config::builder()
        .default_read_policy(ReadPolicy::builder().max_interpolation_expansions(0).build())
        .build();
    limited.set("base", "8080").unwrap();
    limited.set("ports", vec!["${base}", "${missing}"]).unwrap();
    let error = limited.get_interpolated::<u16>("ports").unwrap_err();
    assert!(
        matches!(
            error,
            ConfigError::SubstitutionExpansionLimitExceeded { ref path, max_expansions: 0 } if path == "ports"
        ),
        "{error:?}"
    );

    let limits = qubit_datatype::ConversionLimits::builder()
        .operation_limits(
            qubit_datatype::ConversionOperationLimits::builder()
                .max_output_bytes(3)
                .build(),
        )
        .build();
    let mut output_limited = Config::builder()
        .default_read_policy(ReadPolicy::builder().conversion_limits(limits).build())
        .build();
    output_limited.set("base", "8080").unwrap();
    output_limited.set("ports", vec!["${base}", "${missing}"]).unwrap();
    let error = output_limited.get_interpolated::<String>("ports").unwrap_err();
    assert!(
        matches!(error, ConfigError::ConversionError { ref key, .. } if key == "base"),
        "{error:?}"
    );
}

#[test]
fn test_interpolated_collection_blank_policy_error_stops_candidate_fallback() {
    let mut config = Config::builder()
        .default_read_policy(
            ReadPolicy::builder()
                .blank_string_policy(BlankStringPolicy::TreatAsMissing)
                .build(),
        )
        .build();
    config.set("blank", "").unwrap();
    config.set("fallback", "8080").unwrap();
    config.set("primary", vec!["${blank}", "${missing}"]).unwrap();

    let error = config
        .get_optional_any_interpolated::<u16>(["primary", "fallback"])
        .unwrap_err();
    assert!(
        matches!(
            error,
            ConfigError::ValueError { ref key, ref source } if key == "primary"
                && matches!(source, ValueError::Missing(missing) if missing.source_index() == Some(0))
        ),
        "{error:?}"
    );
}

#[test]
fn test_get_optional_interpolated_and_or_handle_missing_values() {
    let config = Config::new();

    let optional = config
        .get_optional_interpolated::<String>("missing")
        .expect("read missing interpolated value");
    let default = config
        .get_interpolated_or::<u16>("missing", 8080)
        .expect("read interpolated default");

    assert_eq!(optional, None);
    assert_eq!(default, 8080);
}

#[test]
fn test_get_any_interpolated_uses_alias_order() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("URL", "http://${host}/api").expect("set URL");

    let value = config
        .get_any_interpolated::<String>(["url", "URL"])
        .expect("read interpolated alias");

    assert_eq!(value, "http://localhost/api");
}

#[test]
fn test_get_optional_any_interpolated_and_or_handle_missing_values() {
    let config = Config::new();

    let optional = config
        .get_optional_any_interpolated::<String>(["url", "URL"])
        .expect("read missing interpolated aliases");
    let default = config
        .get_any_interpolated_or::<u16>(["port", "PORT"], 8080)
        .expect("read interpolated alias default");

    assert_eq!(optional, None);
    assert_eq!(default, 8080);
}

#[test]
fn test_deserialize_is_raw_and_deserialize_interpolated_is_explicit() {
    let mut config = Config::new();
    config.set("host", "localhost").expect("set host");
    config.set("base_port", "8080").expect("set base port");
    config.set("raw.url", "http://${host}/api").expect("set raw URL");
    config
        .set("endpoint.url", "http://${host}/api")
        .expect("set endpoint URL");
    config.set("endpoint.port", "${base_port}").expect("set endpoint port");

    let raw = config
        .deserialize::<RawEndpoint>("raw")
        .expect("deserialize raw endpoint");
    let interpolated = config
        .deserialize_with::<Endpoint>(
            "endpoint",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Reject,
            },
        )
        .expect("deserialize interpolated endpoint");

    assert_eq!(raw.url, "http://${host}/api");
    assert_eq!(
        interpolated,
        Endpoint {
            url: "http://localhost/api".to_string(),
            port: 8080,
        },
    );
}

#[test]
fn test_interpolated_reads_keep_environment_outside_the_trust_boundary() {
    const VARIABLE: &str = "QUBIT_CONFIG_EXPLICIT_TRUST_BOUNDARY";
    const CHILD: &str = "QUBIT_CONFIG_EXPLICIT_TRUST_CHILD";
    const NAME: &str = "test_interpolated_reads_keep_environment_outside_the_trust_boundary";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().expect("test executable path"))
            .arg("--exact")
            .arg(NAME)
            .arg("--nocapture")
            .env(CHILD, "1")
            .env(VARIABLE, "from-env")
            .output()
            .expect("child test process should start");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
        assert!(output.status.success(), "{stdout}\n{stderr}");
        return;
    }

    let mut config = Config::new();
    config
        .set_default_read_policy(ReadPolicy::env_friendly())
        .set("value", "${QUBIT_CONFIG_EXPLICIT_TRUST_BOUNDARY}")
        .expect("set interpolated value");

    let default_result = config.get_interpolated::<String>("value");
    assert!(matches!(
        default_result,
        Err(ConfigError::SubstitutionError { path, .. }) if path == "value"
    ));

    let trusted_policy = ReadPolicy::builder()
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    assert_eq!(
        config
            .read_with(&trusted_policy)
            .get_interpolated::<String>("value")
            .unwrap(),
        "from-env"
    );
}
