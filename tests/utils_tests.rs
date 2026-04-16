/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Utility Function Tests
//!
//! Tests configuration utility functions, including variable substitution.
//!
//! # Author
//!
//! Haixing Hu

use qubit_config::{substitute_variables, Config, ConfigError};

#[test]
fn test_substitute_simple() {
    let mut config = Config::new();
    config.set("name", "world").unwrap();

    let result = substitute_variables("Hello, ${name}!", &config, 10).unwrap();
    assert_eq!(result, "Hello, world!");
}

#[test]
fn test_substitute_multiple() {
    let mut config = Config::new();
    config.set("host", "localhost").unwrap();
    config.set("port", "8080").unwrap();

    let result = substitute_variables("http://${host}:${port}/api", &config, 10).unwrap();
    assert_eq!(result, "http://localhost:8080/api");
}

#[test]
fn test_substitute_repeated_placeholder() {
    let mut config = Config::new();
    config.set("name", "world").unwrap();

    let result = substitute_variables("${name}-${name}-${name}", &config, 10).unwrap();
    assert_eq!(result, "world-world-world");
}

#[test]
fn test_substitute_recursive() {
    let mut config = Config::new();
    config.set("a", "value_a").unwrap();
    config.set("b", "${a}_b").unwrap();
    config.set("c", "${b}_c").unwrap();

    let result = substitute_variables("${c}", &config, 10).unwrap();
    assert_eq!(result, "value_a_b_c");
}

#[test]
fn test_substitute_depth_exceeded() {
    let mut config = Config::new();
    config.set("a", "${b}").unwrap();
    config.set("b", "${a}").unwrap();

    let result = substitute_variables("${a}", &config, 5);
    assert!(matches!(
        result,
        Err(ConfigError::SubstitutionDepthExceeded(5))
    ));
}

#[test]
fn test_substitute_env_var() {
    std::env::set_var("TEST_VAR", "test_value");

    let config = Config::new();
    let result = substitute_variables("Value: ${TEST_VAR}", &config, 10).unwrap();
    assert_eq!(result, "Value: test_value");

    std::env::remove_var("TEST_VAR");
}

#[test]
fn test_substitute_empty_string() {
    let config = Config::new();
    let result = substitute_variables("", &config, 10).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_substitute_zero_depth_without_placeholders_should_succeed() {
    let config = Config::new();
    let result = substitute_variables("plain text", &config, 0).unwrap();
    assert_eq!(result, "plain text");
}

#[test]
fn test_substitute_variable_not_found() {
    let config = Config::new();
    let result = substitute_variables("${NONEXISTENT_VAR}", &config, 10);
    assert!(matches!(result, Err(ConfigError::SubstitutionError(_))));

    if let Err(ConfigError::SubstitutionError(msg)) = result {
        assert!(msg.contains("Cannot resolve variable: NONEXISTENT_VAR"));
    }
}

#[test]
fn test_substitute_no_variables() {
    let config = Config::new();
    let result = substitute_variables("Plain text with no variables", &config, 10).unwrap();
    assert_eq!(result, "Plain text with no variables");
}

#[test]
fn test_substitute_mixed_sources() {
    std::env::set_var("ENV_VAR", "from_env");

    let mut config = Config::new();
    config.set("CONFIG_VAR", "from_config").unwrap();

    let result = substitute_variables("${CONFIG_VAR} and ${ENV_VAR}", &config, 10).unwrap();
    assert_eq!(result, "from_config and from_env");

    std::env::remove_var("ENV_VAR");
}

#[test]
fn test_substitute_config_priority_over_env() {
    std::env::set_var("SHARED_VAR", "from_env");

    let mut config = Config::new();
    config.set("SHARED_VAR", "from_config").unwrap();

    // Config should have priority over environment
    let result = substitute_variables("${SHARED_VAR}", &config, 10).unwrap();
    assert_eq!(result, "from_config");

    std::env::remove_var("SHARED_VAR");
}

#[test]
fn test_substitute_does_not_fallback_to_env_on_config_type_error() {
    std::env::set_var("STRICT_VAR", "from_env");

    let mut config = Config::new();
    config.set("STRICT_VAR", 8080i32).unwrap();

    let result = substitute_variables("${STRICT_VAR}", &config, 10);
    assert!(matches!(
        result,
        Err(ConfigError::TypeMismatch { .. }) | Err(ConfigError::ConversionError { .. })
    ));

    std::env::remove_var("STRICT_VAR");
}
