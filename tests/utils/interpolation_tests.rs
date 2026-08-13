// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Config;
use crate::ConfigError;
use crate::ReadPolicy;
#[test]
fn test_get_string_substitutes_simple_placeholder() {
    let mut config = Config::new();
    config.set("name", "world").unwrap();
    config.set("greeting", "Hello, ${name}!").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("greeting").unwrap(),
        "Hello, world!"
    );
}

#[test]
fn test_get_string_substitutes_multiple_placeholders() {
    let mut config = Config::new();
    config.set("host", "localhost").unwrap();
    config.set("port", "8080").unwrap();
    config.set("url", "http://${host}:${port}/api").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("url").unwrap(),
        "http://localhost:8080/api"
    );
}

#[test]
fn test_get_string_substitutes_repeated_placeholder() {
    let mut config = Config::new();
    config.set("name", "world").unwrap();
    config.set("value", "${name}-${name}-${name}").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("value").unwrap(),
        "world-world-world"
    );
}

#[test]
fn test_get_string_substitutes_recursively() {
    let mut config = Config::new();
    config.set("a", "value_a").unwrap();
    config.set("b", "${a}_b").unwrap();
    config.set("c", "${b}_c").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("c").unwrap(),
        "value_a_b_c"
    );
}

#[test]
fn test_get_string_rejects_too_many_substitution_expansions() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_expansions(2));
    config.set("name", "world").unwrap();
    config.set("value", "${name}-${name}-${name}").unwrap();

    let result = config.get_interpolated::<String>("value");

    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionExpansionLimitExceeded {
            path,
            max_expansions: 2,
        }) if path == "value"
    ));
}

#[test]
fn test_get_string_accepts_substitution_expansions_at_limit() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_expansions(2));
    config.set("first", "one").unwrap();
    config.set("second", "two").unwrap();
    config.set("value", "${first}-${second}").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("value").unwrap(),
        "one-two"
    );
}

#[test]
fn test_get_string_rejects_substitution_output_over_byte_limit() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_output_bytes(7));
    config.set("part", "1234").unwrap();
    config.set("value", "${part}${part}").unwrap();

    let result = config.get_interpolated::<String>("value");

    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionOutputTooLarge {
            path,
            max_output_bytes: 7,
        }) if path == "value"
    ));
}

#[test]
fn test_get_string_accepts_substitution_output_at_byte_limit() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_output_bytes(16));
    config.set("part", "1234").unwrap();
    config.set("value", "${part}${part}").unwrap();

    let value = config.get_interpolated::<String>("value").unwrap();

    assert_eq!(value, "12341234");
}

#[test]
fn test_get_string_accepts_exact_nested_output_without_double_charging() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_output_bytes(4));
    config.set("part", "1234").unwrap();
    config.set("value", "${part}").unwrap();

    assert_eq!(config.get_interpolated::<String>("value").unwrap(), "1234");
}

#[test]
fn test_get_string_rejects_oversized_nested_intermediate_output() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_output_bytes(4));
    config.set("part", "12345").unwrap();
    config.set("value", "${part}").unwrap();

    assert!(matches!(
        config.get_interpolated::<String>("value"),
        Err(ConfigError::SubstitutionOutputTooLarge {
            path,
            max_output_bytes: 4,
        }) if path == "value"
    ));
}

#[test]
fn test_get_string_rejects_oversized_final_concatenation() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default().with_max_interpolation_output_bytes(4));
    config.set("part", "12").unwrap();
    config.set("value", "x${part}yz").unwrap();

    assert!(matches!(
        config.get_interpolated::<String>("value"),
        Err(ConfigError::SubstitutionOutputTooLarge {
            path,
            max_output_bytes: 4,
        }) if path == "value"
    ));
}

#[test]
fn test_get_string_substitution_depth_exceeded() {
    let mut config = Config::new();
    crate::set_max_interpolation_depth(&mut config, 5);
    config.set("a", "${b}").unwrap();
    config.set("b", "${c}").unwrap();
    config.set("c", "${d}").unwrap();
    config.set("d", "${e}").unwrap();
    config.set("e", "${f}").unwrap();
    config.set("f", "${g}").unwrap();
    config.set("g", "done").unwrap();

    let result = config.get_interpolated::<String>("a");

    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionDepthExceeded {
            path,
            max_depth: 5,
        }) if path == "a"
    ));
}

#[test]
fn test_get_string_uses_environment_fallback() {
    unsafe {
        std::env::set_var("QUBIT_CONFIG_TEST_ENV_VAR", "test_value");
    }
    let mut config = Config::new();
    config.set_default_read_policy(crate::with_environment_fallback(ReadPolicy::default()));
    config
        .set("value", "Value: ${QUBIT_CONFIG_TEST_ENV_VAR}")
        .unwrap();

    let result = config.get_interpolated::<String>("value");

    unsafe {
        std::env::remove_var("QUBIT_CONFIG_TEST_ENV_VAR");
    }
    assert_eq!(result.unwrap(), "Value: test_value");
}

#[test]
fn test_get_string_can_disable_environment_fallback() {
    unsafe {
        std::env::set_var("QUBIT_CONFIG_TEST_ENV_DISABLED", "from_env");
    }
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::default());
    config
        .set("value", "${QUBIT_CONFIG_TEST_ENV_DISABLED}")
        .unwrap();

    let result = config.get_interpolated::<String>("value");

    unsafe {
        std::env::remove_var("QUBIT_CONFIG_TEST_ENV_DISABLED");
    }
    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionError { path, .. })
            if path == "value"
    ));
}

#[test]
fn test_get_string_empty_string_succeeds() {
    let mut config = Config::new();
    config.set("empty", "").unwrap();

    assert_eq!(config.get_interpolated::<String>("empty").unwrap(), "");
}

#[test]
fn test_get_string_zero_depth_without_placeholders_succeeds() {
    let mut config = Config::new();
    crate::set_max_interpolation_depth(&mut config, 0);
    config.set("plain", "plain text").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("plain").unwrap(),
        "plain text"
    );
}

#[test]
fn test_get_string_unresolved_variable_returns_error() {
    let mut config = Config::new();
    config
        .set(
            "missing",
            "${QUBIT_CONFIG_TEST_VAR_THAT_MUST_NOT_EXIST_001}",
        )
        .unwrap();

    let err = config
        .get_interpolated::<String>("missing")
        .expect_err("unresolved variable should return an error");

    assert!(matches!(
        err,
        ConfigError::SubstitutionError { ref path, .. }
            if path == "missing"
    ));
    assert!(
        err.to_string()
            .contains("QUBIT_CONFIG_TEST_VAR_THAT_MUST_NOT_EXIST_001")
    );
}

#[test]
fn test_get_string_list_unresolved_variable_returns_error() {
    let mut config = Config::new();
    config
        .set(
            "values",
            "${QUBIT_CONFIG_TEST_LIST_VAR_THAT_MUST_NOT_EXIST_001}",
        )
        .unwrap();

    let err = config
        .get_interpolated::<Vec<String>>("values")
        .expect_err("unresolved variable in list read should return an error");

    assert!(matches!(
        err,
        ConfigError::SubstitutionError { ref path, .. }
            if path == "values"
    ));
    assert!(
        err.to_string()
            .contains("QUBIT_CONFIG_TEST_LIST_VAR_THAT_MUST_NOT_EXIST_001")
    );
}

#[test]
fn test_get_string_without_variables_returns_original_value() {
    let mut config = Config::new();
    config.set("plain", "Plain text with no variables").unwrap();

    assert_eq!(
        config.get_interpolated::<String>("plain").unwrap(),
        "Plain text with no variables"
    );
}

#[test]
fn test_get_string_uses_config_and_environment_sources() {
    unsafe {
        std::env::set_var("QUBIT_CONFIG_TEST_ENV_SOURCE", "from_env");
    }
    let mut config = Config::new();
    config.set_default_read_policy(crate::with_environment_fallback(ReadPolicy::default()));
    config.set("CONFIG_SOURCE", "from_config").unwrap();
    config
        .set(
            "combined",
            "${CONFIG_SOURCE} and ${QUBIT_CONFIG_TEST_ENV_SOURCE}",
        )
        .unwrap();

    let result = config.get_interpolated::<String>("combined");

    unsafe {
        std::env::remove_var("QUBIT_CONFIG_TEST_ENV_SOURCE");
    }
    assert_eq!(result.unwrap(), "from_config and from_env");
}

#[test]
fn test_get_string_config_value_has_priority_over_environment() {
    unsafe {
        std::env::set_var("QUBIT_CONFIG_TEST_SHARED_VAR", "from_env");
    }
    let mut config = Config::new();
    config.set_default_read_policy(crate::with_environment_fallback(ReadPolicy::default()));
    config
        .set("QUBIT_CONFIG_TEST_SHARED_VAR", "from_config")
        .unwrap();
    config
        .set("value", "${QUBIT_CONFIG_TEST_SHARED_VAR}")
        .unwrap();

    let result = config.get_interpolated::<String>("value");

    unsafe {
        std::env::remove_var("QUBIT_CONFIG_TEST_SHARED_VAR");
    }
    assert_eq!(result.unwrap(), "from_config");
}

#[test]
fn test_get_string_converts_config_value_instead_of_environment() {
    unsafe {
        std::env::set_var("QUBIT_CONFIG_TEST_STRICT_VAR", "from_env");
    }
    let mut config = Config::new();
    config.set_default_read_policy(crate::with_environment_fallback(ReadPolicy::default()));
    config.set("QUBIT_CONFIG_TEST_STRICT_VAR", 8080i32).unwrap();
    config
        .set("value", "${QUBIT_CONFIG_TEST_STRICT_VAR}")
        .unwrap();

    let result = config.get_interpolated::<String>("value");

    unsafe {
        std::env::remove_var("QUBIT_CONFIG_TEST_STRICT_VAR");
    }
    assert_eq!(result.unwrap(), "8080");
}

#[test]
fn test_get_string_environment_fallback_reports_missing_env_var() {
    let mut config = Config::new();
    config.set_default_read_policy(crate::with_environment_fallback(ReadPolicy::default()));
    config
        .set("value", "${QUBIT_CONFIG_TEST_ENV_MISSING_FOR_FALLBACK}")
        .unwrap();

    let result = config.get_interpolated::<String>("value");

    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionError { path, message })
            if path == "value"
                && message.contains("QUBIT_CONFIG_TEST_ENV_MISSING_FOR_FALLBACK")
    ));
}

#[test]
fn test_get_string_substitution_cycle_reports_variable_chain() {
    let mut config = Config::new();
    config.set("a", "${b}").unwrap();
    config.set("b", "${c}").unwrap();
    config.set("c", "${b}").unwrap();

    let error = config.get_interpolated::<String>("a").unwrap_err();

    assert!(matches!(
        &error,
        ConfigError::SubstitutionCycle { path, chain }
            if path == "a"
                && chain == &vec!["b".to_string(), "c".to_string(), "b".to_string()]
    ));
    assert_eq!(error.path(), Some("a"));
}
