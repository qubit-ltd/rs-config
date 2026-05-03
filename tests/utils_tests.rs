/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Utility Function Tests
//!
//! Integration tests for deserialize JSON building (`property_to_json_value` /
//! dotted-key insertion). Variable substitution is covered by unit tests in
//! `src/utils.rs`.
//!

use qubit_config::{Config, ConfigError, Property, options::ConfigReadOptions};
use qubit_datatype::DataType;
use qubit_value::MultiValues;
use serde::Deserialize;
use std::collections::HashMap;

// ============================================================================
// Config::deserialize() (utils: property_to_json_value, insert_deserialize_value)
// ============================================================================

#[cfg(test)]
mod test_deserialize {
    #[allow(unused_imports)]
    use super::{
        Config, ConfigError, ConfigReadOptions, DataType, Deserialize, HashMap, MultiValues,
        Property,
    };

    #[derive(Deserialize, Debug, PartialEq)]
    struct ServerConfig {
        host: String,
        port: i32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DatabaseConfig {
        url: String,
        pool: i32,
        timeout: Option<i64>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct AppConfig {
        name: String,
        version: String,
        debug: bool,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedServerConfig {
        host: String,
        port: i32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedAppConfig {
        server: NestedServerConfig,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct WithOptionals {
        host: String,
        port: Option<i32>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct WithDefault {
        host: String,
        #[serde(default = "default_port")]
        port: i32,
    }

    /// Returns the default port used by serde default tests.
    fn default_port() -> i32 {
        8080
    }

    #[test]
    fn test_deserialize_basic_struct() {
        let mut config = Config::new();
        config.set("server.host", "localhost").unwrap();
        config.set("server.port", 8080).unwrap();

        let server: ServerConfig = config.deserialize("server").unwrap();
        assert_eq!(server.host, "localhost");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_deserialize_with_optional_present() {
        let mut config = Config::new();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 10).unwrap();
        config.set("db.timeout", 30i64).unwrap();

        let db: DatabaseConfig = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 10);
        assert_eq!(db.timeout, Some(30));
    }

    #[test]
    fn test_deserialize_with_optional_absent() {
        let mut config = Config::new();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 10).unwrap();

        let db: DatabaseConfig = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 10);
        assert_eq!(db.timeout, None);
    }

    #[test]
    fn test_deserialize_bool_field() {
        let mut config = Config::new();
        config.set("app.name", "MyApp").unwrap();
        config.set("app.version", "1.0.0").unwrap();
        config.set("app.debug", true).unwrap();

        let app: AppConfig = config.deserialize("app").unwrap();
        assert_eq!(app.name, "MyApp");
        assert_eq!(app.version, "1.0.0");
        assert!(app.debug);
    }

    #[test]
    fn test_deserialize_nested_struct() {
        let mut config = Config::new();
        config.set("app.server.host", "localhost").unwrap();
        config.set("app.server.port", 9090).unwrap();

        let app: NestedAppConfig = config.deserialize("app").unwrap();
        assert_eq!(app.server.host, "localhost");
        assert_eq!(app.server.port, 9090);
    }

    #[test]
    fn test_deserialize_empty_prefix() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", 8080).unwrap();

        let server: ServerConfig = config.deserialize("").unwrap();
        assert_eq!(server.host, "localhost");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_deserialize_missing_required_field_returns_error() {
        let mut config = Config::new();
        config.set("server.host", "localhost").unwrap();
        // Missing "port"

        let result: Result<ServerConfig, _> = config.deserialize("server");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::DeserializeError { path, .. } => {
                assert_eq!(path, "server");
            }
            e => panic!("Expected DeserializeError, got {:?}", e),
        }
    }

    #[test]
    fn test_deserialize_with_optional_null_field() {
        let mut config = Config::new();
        config.set("srv.host", "localhost").unwrap();
        // Insert null for port
        config.set_null("srv.port", DataType::Int32).unwrap();

        let result: WithOptionals = config.deserialize("srv").unwrap();
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, None);
    }

    #[test]
    fn test_deserialize_blank_field_with_missing_policy_behaves_as_absent() {
        let mut config = Config::new().with_read_options(ConfigReadOptions::env_friendly());
        config.set("srv.host", "localhost").unwrap();
        config.set("srv.port", "   ").unwrap();

        let optional: WithOptionals = config.deserialize("srv").unwrap();
        assert_eq!(optional.port, None);

        let defaulted: WithDefault = config.deserialize("srv").unwrap();
        assert_eq!(defaulted.port, 8080);
    }

    #[test]
    fn test_deserialize_hashmap() {
        let mut config = Config::new();
        config.set("headers.authorization", "Bearer token").unwrap();
        config
            .set("headers.content-type", "application/json")
            .unwrap();

        let headers: HashMap<String, String> = config.deserialize("headers").unwrap();
        assert_eq!(
            headers.get("authorization"),
            Some(&"Bearer token".to_string())
        );
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_deserialize_conflicting_dotted_key_returns_key_conflict() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct CtxConfig {
            a: i32,
        }

        let mut config = Config::new();
        config.set("ctx.a", 1).unwrap();
        config.set("ctx.a.b", "conflict").unwrap();

        let result = config.deserialize::<CtxConfig>("ctx");
        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "a"
        ));
    }

    #[test]
    fn test_deserialize_conflicting_dotted_key_does_not_keep_flat_fallback() {
        let mut config = Config::new();
        config.set("ctx.a", 1).unwrap();
        config.set("ctx.a.b", "conflict").unwrap();

        let result = config.deserialize::<HashMap<String, serde_json::Value>>("ctx");
        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "a"
        ));
    }

    #[test]
    fn test_deserialize_malformed_dotted_key_returns_key_conflict() {
        let mut config = Config::new();
        config.set("bad..key", "value").unwrap();

        let result = config.deserialize::<HashMap<String, serde_json::Value>>("");
        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "bad..key"
        ));
    }

    #[test]
    fn test_deserialize_exact_json_property_as_root_value() {
        let mut config = Config::new();
        config
            .insert_property(
                "server",
                Property::with_value(
                    "server",
                    MultiValues::Json(vec![serde_json::json!({
                        "host": "localhost",
                        "port": "8080",
                    })]),
                ),
            )
            .unwrap();

        let server: ServerConfig = config.deserialize("server").unwrap();

        assert_eq!(
            server,
            ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
            }
        );
    }

    #[test]
    fn test_deserialize_exact_key_and_subtree_returns_key_conflict() {
        let mut config = Config::new();
        config.set("server", "root").unwrap();
        config.set("server.host", "localhost").unwrap();

        let result = config.deserialize::<ServerConfig>("server");

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "server"
        ));
    }

    #[test]
    fn test_deserialize_multivalue_as_array() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct WithList {
            ports: Vec<i32>,
        }

        let mut config = Config::new();
        config.set("svc.ports", vec![8080, 8081, 8082]).unwrap();

        let svc: WithList = config.deserialize("svc").unwrap();
        assert_eq!(svc.ports, vec![8080, 8081, 8082]);
    }

    #[test]
    fn test_deserialize_substitutes_string_fields_and_lists() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            base_url: String,
            endpoints: Vec<String>,
        }

        let mut config = Config::new();
        config.set("svc.host", "localhost").unwrap();
        config.set("svc.port", "8080").unwrap();
        config
            .set("svc.base_url", "http://${host}:${port}")
            .unwrap();
        config
            .set(
                "svc.endpoints",
                vec!["${base_url}/users", "${base_url}/health"],
            )
            .unwrap();

        let svc: ServiceConfig = config.deserialize("svc").unwrap();
        assert_eq!(svc.base_url, "http://localhost:8080");
        assert_eq!(
            svc.endpoints,
            vec![
                "http://localhost:8080/users".to_string(),
                "http://localhost:8080/health".to_string(),
            ],
        );
    }

    #[test]
    fn test_deserialize_substitutes_root_scope_fallback() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            url: String,
        }

        let mut config = Config::new();
        config.set("base_url", "http://example.test").unwrap();
        config.set("svc.url", "${base_url}/v1").unwrap();

        let svc: ServiceConfig = config.deserialize("svc").unwrap();

        assert_eq!(svc.url, "http://example.test/v1");
    }

    #[test]
    fn test_deserialize_substitution_local_conversion_has_priority_over_root() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            url: String,
        }

        let mut config = Config::new();
        config.set("base_url", "http://example.test").unwrap();
        config.set("svc.base_url", 123i32).unwrap();
        config.set("svc.url", "${base_url}/v1").unwrap();

        let svc = config.deserialize::<ServiceConfig>("svc").unwrap();

        assert_eq!(svc.url, "123/v1");
    }

    #[test]
    fn test_deserialize_uses_config_conversion_for_string_scalars() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            port: u16,
            enabled: bool,
        }

        let mut config = Config::new();
        config.set("svc.port", "8080").unwrap();
        config.set("svc.enabled", "1").unwrap();

        let svc = config.deserialize::<ServiceConfig>("svc").unwrap();

        assert_eq!(
            svc,
            ServiceConfig {
                port: 8080,
                enabled: true,
            }
        );
    }

    #[test]
    fn test_deserialize_uses_read_options_for_env_style_values() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            enabled: bool,
            ports: Vec<u16>,
        }

        let mut config = Config::new();
        config.set_read_options(ConfigReadOptions::env_friendly());
        config.set("svc.enabled", "yes").unwrap();
        config.set("svc.ports", "8080, 8081,,8082").unwrap();

        let svc = config.deserialize::<ServiceConfig>("svc").unwrap();

        assert_eq!(
            svc,
            ServiceConfig {
                enabled: true,
                ports: vec![8080, 8081, 8082],
            }
        );
    }

    #[test]
    fn test_deserialize_substitutes_nested_json_strings() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            meta: serde_json::Value,
        }

        let mut config = Config::new();
        config.set("svc.host", "localhost").unwrap();
        config.set("svc.base_url", "http://${host}").unwrap();
        config
            .insert_property(
                "svc.meta",
                Property::with_value(
                    "svc.meta",
                    MultiValues::Json(vec![serde_json::json!({
                        "enabled": true,
                        "tags": ["${host}", "static"],
                        "url": "${base_url}/v1",
                    })]),
                ),
            )
            .unwrap();

        let svc: ServiceConfig = config.deserialize("svc").unwrap();
        assert_eq!(
            svc.meta,
            serde_json::json!({
                "enabled": true,
                "tags": ["localhost", "static"],
                "url": "http://localhost/v1",
            }),
        );
    }

    #[test]
    fn test_deserialize_respects_substitution_disabled() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            url: String,
        }

        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set("svc.host", "localhost").unwrap();
        config.set("svc.url", "http://${host}").unwrap();

        let svc: ServiceConfig = config.deserialize("svc").unwrap();
        assert_eq!(svc.url, "http://${host}");
    }

    #[test]
    fn test_deserialize_unresolved_variable_returns_substitution_error() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ServiceConfig {
            url: String,
        }

        let mut config = Config::new();
        config
            .set("svc.url", "${QUBIT_CONFIG_UNSET_DESERIALIZE_VAR_12345}")
            .unwrap();

        let err = config
            .deserialize::<ServiceConfig>("svc")
            .expect_err("unresolved variable should fail before serde deserialization");
        match err {
            ConfigError::SubstitutionError(msg) => {
                assert!(msg.contains("QUBIT_CONFIG_UNSET_DESERIALIZE_VAR_12345"));
            }
            other => panic!("Expected SubstitutionError, got {:?}", other),
        }
    }
}

// ============================================================================
// Variable substitution coverage through public Config readers
// ============================================================================

#[cfg(test)]
mod test_variable_substitution {
    #[allow(unused_imports)]
    use super::{
        Config, ConfigError, ConfigReadOptions, DataType, Deserialize, HashMap, MultiValues,
        Property,
    };

    #[test]
    fn test_get_string_substitutes_simple_placeholder() {
        let mut config = Config::new();
        config.set("name", "world").unwrap();
        config.set("greeting", "Hello, ${name}!").unwrap();

        assert_eq!(config.get_string("greeting").unwrap(), "Hello, world!");
    }

    #[test]
    fn test_get_string_substitutes_multiple_placeholders() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", "8080").unwrap();
        config.set("url", "http://${host}:${port}/api").unwrap();

        assert_eq!(
            config.get_string("url").unwrap(),
            "http://localhost:8080/api"
        );
    }

    #[test]
    fn test_get_string_substitutes_repeated_placeholder() {
        let mut config = Config::new();
        config.set("name", "world").unwrap();
        config.set("value", "${name}-${name}-${name}").unwrap();

        assert_eq!(config.get_string("value").unwrap(), "world-world-world");
    }

    #[test]
    fn test_get_string_substitutes_recursively() {
        let mut config = Config::new();
        config.set("a", "value_a").unwrap();
        config.set("b", "${a}_b").unwrap();
        config.set("c", "${b}_c").unwrap();

        assert_eq!(config.get_string("c").unwrap(), "value_a_b_c");
    }

    #[test]
    fn test_get_string_substitution_depth_exceeded() {
        let mut config = Config::new();
        config.set_max_substitution_depth(5);
        config.set("a", "${b}").unwrap();
        config.set("b", "${c}").unwrap();
        config.set("c", "${d}").unwrap();
        config.set("d", "${e}").unwrap();
        config.set("e", "${f}").unwrap();
        config.set("f", "${g}").unwrap();
        config.set("g", "done").unwrap();

        let result = config.get_string("a");

        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded(5))
        ));
    }

    #[test]
    fn test_get_string_uses_environment_fallback() {
        unsafe {
            std::env::set_var("QUBIT_CONFIG_TEST_ENV_VAR", "test_value");
        }
        let mut config = Config::new();
        config.set_read_options(
            ConfigReadOptions::default().with_env_variable_substitution_enabled(true),
        );
        config
            .set("value", "Value: ${QUBIT_CONFIG_TEST_ENV_VAR}")
            .unwrap();

        let result = config.get_string("value");

        unsafe {
            std::env::remove_var("QUBIT_CONFIG_TEST_ENV_VAR");
        }
        assert_eq!(result.unwrap(), "Value: test_value");
    }

    #[test]
    fn test_get_string_does_not_use_environment_fallback_by_default() {
        unsafe {
            std::env::set_var("QUBIT_CONFIG_TEST_ENV_DISABLED", "from_env");
        }
        let mut config = Config::new();
        config
            .set("value", "${QUBIT_CONFIG_TEST_ENV_DISABLED}")
            .unwrap();

        let result = config.get_string("value");

        unsafe {
            std::env::remove_var("QUBIT_CONFIG_TEST_ENV_DISABLED");
        }
        assert!(matches!(result, Err(ConfigError::SubstitutionError(_))));
    }

    #[test]
    fn test_get_string_empty_string_succeeds() {
        let mut config = Config::new();
        config.set("empty", "").unwrap();

        assert_eq!(config.get_string("empty").unwrap(), "");
    }

    #[test]
    fn test_get_string_zero_depth_without_placeholders_succeeds() {
        let mut config = Config::new();
        config.set_max_substitution_depth(0);
        config.set("plain", "plain text").unwrap();

        assert_eq!(config.get_string("plain").unwrap(), "plain text");
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
            .get_string("missing")
            .expect_err("unresolved variable should return an error");

        assert!(matches!(err, ConfigError::SubstitutionError(_)));
        assert!(
            err.to_string()
                .contains("QUBIT_CONFIG_TEST_VAR_THAT_MUST_NOT_EXIST_001")
        );
    }

    #[test]
    fn test_get_string_without_variables_returns_original_value() {
        let mut config = Config::new();
        config.set("plain", "Plain text with no variables").unwrap();

        assert_eq!(
            config.get_string("plain").unwrap(),
            "Plain text with no variables"
        );
    }

    #[test]
    fn test_get_string_uses_config_and_environment_sources() {
        unsafe {
            std::env::set_var("QUBIT_CONFIG_TEST_ENV_SOURCE", "from_env");
        }
        let mut config = Config::new();
        config.set_read_options(
            ConfigReadOptions::default().with_env_variable_substitution_enabled(true),
        );
        config.set("CONFIG_SOURCE", "from_config").unwrap();
        config
            .set(
                "combined",
                "${CONFIG_SOURCE} and ${QUBIT_CONFIG_TEST_ENV_SOURCE}",
            )
            .unwrap();

        let result = config.get_string("combined");

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
        config.set_read_options(
            ConfigReadOptions::default().with_env_variable_substitution_enabled(true),
        );
        config
            .set("QUBIT_CONFIG_TEST_SHARED_VAR", "from_config")
            .unwrap();
        config
            .set("value", "${QUBIT_CONFIG_TEST_SHARED_VAR}")
            .unwrap();

        let result = config.get_string("value");

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
        config.set_read_options(
            ConfigReadOptions::default().with_env_variable_substitution_enabled(true),
        );
        config.set("QUBIT_CONFIG_TEST_STRICT_VAR", 8080i32).unwrap();
        config
            .set("value", "${QUBIT_CONFIG_TEST_STRICT_VAR}")
            .unwrap();

        let result = config.get_string("value");

        unsafe {
            std::env::remove_var("QUBIT_CONFIG_TEST_STRICT_VAR");
        }
        assert_eq!(result.unwrap(), "8080");
    }

    #[test]
    fn test_get_string_environment_fallback_reports_missing_env_var() {
        let mut config = Config::new();
        config.set_read_options(
            ConfigReadOptions::default().with_env_variable_substitution_enabled(true),
        );
        config
            .set("value", "${QUBIT_CONFIG_TEST_ENV_MISSING_FOR_FALLBACK}")
            .unwrap();

        let result = config.get_string("value");

        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionError(message))
                if message.contains("QUBIT_CONFIG_TEST_ENV_MISSING_FOR_FALLBACK")
        ));
    }

    #[test]
    fn test_get_string_substitution_cycle_reports_variable_chain() {
        let mut config = Config::new();
        config.set("a", "${b}").unwrap();
        config.set("b", "${c}").unwrap();
        config.set("c", "${b}").unwrap();

        let result = config.get_string("a");

        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionCycle { chain })
                if chain == vec!["b".to_string(), "c".to_string(), "b".to_string()]
        ));
    }
}

// ============================================================================
// property_to_json_value coverage: various MultiValues types via deserialize
// ============================================================================

#[cfg(test)]
mod test_property_to_json_value_coverage {
    #[allow(unused_imports)]
    use super::{Config, ConfigError, DataType, Deserialize, HashMap, MultiValues, Property};
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use num_bigint::BigInt;
    use std::str::FromStr;
    use std::time::Duration;
    use url::Url;

    #[derive(Deserialize, Debug, PartialEq)]
    struct AnyStruct {
        val: serde_json::Value,
    }

    fn config_with_mv(key: &str, mv: MultiValues) -> Config {
        let mut config = Config::new();
        config
            .insert_property(key, Property::with_value(key, mv))
            .unwrap();
        config
    }

    #[test]
    fn test_deserialize_bool_single() {
        let mut config = Config::new();
        config.set("x.val", true).unwrap();
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_deserialize_bool_multi() {
        let mut config = Config::new();
        config.set("x.val", vec![true, false]).unwrap();
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_int8() {
        let config = config_with_mv("x.val", MultiValues::Int8(vec![42i8]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_int16() {
        let config = config_with_mv("x.val", MultiValues::Int16(vec![1000i16]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(1000));
    }

    #[test]
    fn test_deserialize_int32() {
        let config = config_with_mv("x.val", MultiValues::Int32(vec![8080i32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(8080));
    }

    #[test]
    fn test_deserialize_int64() {
        let config = config_with_mv("x.val", MultiValues::Int64(vec![9999i64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(9999));
    }

    #[test]
    fn test_deserialize_intsize() {
        let config = config_with_mv("x.val", MultiValues::IntSize(vec![42isize]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uint8() {
        let config = config_with_mv("x.val", MultiValues::UInt8(vec![255u8]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(255));
    }

    #[test]
    fn test_deserialize_uint16() {
        let config = config_with_mv("x.val", MultiValues::UInt16(vec![1000u16]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(1000));
    }

    #[test]
    fn test_deserialize_uint32() {
        let config = config_with_mv("x.val", MultiValues::UInt32(vec![42u32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uint64() {
        let config = config_with_mv("x.val", MultiValues::UInt64(vec![42u64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uintsize() {
        let config = config_with_mv("x.val", MultiValues::UIntSize(vec![42usize]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_float32() {
        let config = config_with_mv("x.val", MultiValues::Float32(vec![1.5f32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_number());
    }

    #[test]
    fn test_deserialize_float64() {
        let config = config_with_mv("x.val", MultiValues::Float64(vec![1.5f64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_number());
    }

    #[test]
    fn test_deserialize_float32_nan_becomes_null() {
        let config = config_with_mv("x.val", MultiValues::Float32(vec![f32::NAN]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_float64_nan_becomes_null() {
        let config = config_with_mv("x.val", MultiValues::Float64(vec![f64::NAN]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_duration() {
        let config = config_with_mv(
            "x.val",
            MultiValues::Duration(vec![Duration::from_millis(500)]),
        );
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!("500ms"));
    }

    #[test]
    fn test_deserialize_url() {
        let url = Url::parse("https://example.com").unwrap();
        let config = config_with_mv("x.val", MultiValues::Url(vec![url]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.as_str().unwrap().contains("example.com"));
    }

    #[test]
    fn test_deserialize_string_map_single() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), "value".to_string());
        let config = config_with_mv("x.val", MultiValues::StringMap(vec![map]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_object());
        assert_eq!(s.val["key"], serde_json::json!("value"));
    }

    #[test]
    fn test_deserialize_string_map_multi() {
        let mut map1 = std::collections::HashMap::new();
        map1.insert("k1".to_string(), "v1".to_string());
        let mut map2 = std::collections::HashMap::new();
        map2.insert("k2".to_string(), "v2".to_string());
        let config = config_with_mv("x.val", MultiValues::StringMap(vec![map1, map2]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_json_single() {
        let json_val = serde_json::json!({"nested": true});
        let config = config_with_mv("x.val", MultiValues::Json(vec![json_val.clone()]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, json_val);
    }

    #[test]
    fn test_deserialize_json_multi() {
        let j1 = serde_json::json!(1);
        let j2 = serde_json::json!(2);
        let config = config_with_mv("x.val", MultiValues::Json(vec![j1, j2]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_char() {
        let config = config_with_mv("x.val", MultiValues::Char(vec!['A']));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!("A"));
    }

    #[test]
    fn test_deserialize_big_integer() {
        let big = BigInt::from(12345678901234567i64);
        let config = config_with_mv("x.val", MultiValues::BigInteger(vec![big]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_big_decimal() {
        let dec = BigDecimal::from_str("3.14159265358979").unwrap();
        let config = config_with_mv("x.val", MultiValues::BigDecimal(vec![dec]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_datetime() {
        let dt = NaiveDateTime::parse_from_str("2026-04-09 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let config = config_with_mv("x.val", MultiValues::DateTime(vec![dt]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_date() {
        let d = NaiveDate::from_ymd_opt(2026, 4, 9).unwrap();
        let config = config_with_mv("x.val", MultiValues::Date(vec![d]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_time() {
        let t = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let config = config_with_mv("x.val", MultiValues::Time(vec![t]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_instant() {
        let instant: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-04-09T12:00:00Z")
            .unwrap()
            .into();
        let config = config_with_mv("x.val", MultiValues::Instant(vec![instant]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_int128() {
        let config = config_with_mv("x.val", MultiValues::Int128(vec![42i128]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_uint128() {
        let config = config_with_mv("x.val", MultiValues::UInt128(vec![42u128]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_empty_multivalue_is_null() {
        let config = config_with_mv("x.val", MultiValues::Empty(DataType::String));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_empty_string_multivalue_is_array() {
        let config = config_with_mv("x.val", MultiValues::String(Vec::new()));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!([]));
    }

    #[test]
    fn test_deserialize_multi_int32_array() {
        let config = config_with_mv("x.val", MultiValues::Int32(vec![1, 2, 3]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
        assert_eq!(s.val.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_deserialize_multi_string_array() {
        let config = config_with_mv(
            "x.val",
            MultiValues::String(vec!["a".to_string(), "b".to_string()]),
        );
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }
}
