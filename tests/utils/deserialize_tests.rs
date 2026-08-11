// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_datatype::BlankStringPolicy;

use crate::Config;
use crate::ConfigError;
use crate::DataType;
use crate::Deserialize;
use crate::HashMap;
use crate::MultiValues;
use crate::Property;
use crate::ReadPolicy;
use crate::Value;
use crate::ValueContainer;

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
    let mut config = Config::new().with_default_read_policy(ReadPolicy::env_friendly());
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
fn test_deserialize_dotted_parent_conflict_reports_scalar_kinds() {
    let cases = [
        (
            ValueContainer::Scalar(Value::Unset(DataType::String)),
            "null",
            "null parent should not be treated as an object",
        ),
        (
            ValueContainer::Scalar(Value::Bool(true)),
            "boolean",
            "boolean parent should not be treated as an object",
        ),
        (
            ValueContainer::Scalar(Value::String("root".to_string())),
            "string",
            "string parent should not be treated as an object",
        ),
        (
            ValueContainer::Collection(MultiValues::Int32(vec![1, 2])),
            "array",
            "array parent should not be treated as an object",
        ),
    ];

    for (parent_value, expected_kind, message) in cases {
        let mut config = Config::new();
        config
            .insert_property("ctx.a", Property::new("ctx.a", parent_value).unwrap())
            .unwrap();
        config.set("ctx.a.b", "conflict").unwrap();

        let result = config.deserialize::<HashMap<String, serde_json::Value>>("ctx");

        assert!(
            matches!(
                result,
                Err(ConfigError::KeyConflict { path, existing, .. })
                    if path == "a" && existing == expected_kind
            ),
            "{message}"
        );
    }
}

#[test]
fn test_deserialize_dotted_child_overrides_same_shape_json_field() {
    let mut config = Config::new();
    config
        .insert_property(
            "ctx.a",
            Property::new(
                "ctx.a",
                Value::Json(serde_json::json!({
                    "b": "from-object",
                    "other": true,
                })),
            )
            .unwrap(),
        )
        .unwrap();
    config.set("ctx.a.b", "from-dotted").unwrap();

    let actual = config
        .deserialize::<HashMap<String, serde_json::Value>>("ctx")
        .unwrap();

    assert_eq!(
        actual.get("a"),
        Some(&serde_json::json!({
            "b": "from-dotted",
            "other": true,
        }))
    );
}

#[test]
fn test_config_rejects_malformed_dotted_key_before_deserialize() {
    let mut config = Config::new();
    let result = config.set("bad..key", "value");
    assert!(matches!(
        result,
        Err(ConfigError::InvalidKey { key, .. }) if key == "bad..key"
    ));
}

#[test]
fn test_deserialize_exact_json_property_as_root_value() {
    let mut config = Config::new();
    config
        .insert_property(
            "server",
            Property::new(
                "server",
                Value::Json(serde_json::json!({
                    "host": "localhost",
                    "port": "8080",
                })),
            )
            .unwrap(),
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

    let svc: ServiceConfig = config.deserialize_interpolated_lenient("svc").unwrap();
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

    let svc: ServiceConfig = config.deserialize_interpolated("svc").unwrap();

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

    let svc = config
        .deserialize_interpolated_lenient::<ServiceConfig>("svc")
        .unwrap();

    assert_eq!(svc.url, "123/v1");
}

#[test]
fn test_deserialize_exact_blank_string_can_be_treated_as_null() {
    let mut config = Config::new();
    config.set_default_read_policy(
        ReadPolicy::env_friendly().with_blank_string_policy(BlankStringPolicy::TreatAsMissing),
    );
    config.set("value", "   ").unwrap();

    let actual: Option<String> = config.deserialize("value").unwrap();

    assert_eq!(actual, None);
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
fn test_deserialize_uses_read_policy_for_env_style_values() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct ServiceConfig {
        enabled: bool,
        ports: Vec<u16>,
    }

    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
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
            Property::new(
                "svc.meta",
                Value::Json(serde_json::json!({
                    "enabled": true,
                    "tags": ["${host}", "static"],
                    "url": "${base_url}/v1",
                })),
            )
            .unwrap(),
        )
        .unwrap();

    let svc: ServiceConfig = config.deserialize_interpolated_lenient("svc").unwrap();
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
fn test_deserialize_preserves_placeholders_by_default() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct ServiceConfig {
        url: String,
    }

    let mut config = Config::new();
    config.set("svc.host", "localhost").unwrap();
    config.set("svc.url", "http://${host}").unwrap();

    let svc: ServiceConfig = config.deserialize_lenient("svc").unwrap();
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
        .deserialize_interpolated::<ServiceConfig>("svc")
        .expect_err("unresolved variable should fail before serde deserialization");
    match err {
        ConfigError::SubstitutionError { path, message } => {
            assert_eq!(path, "svc.url");
            assert!(message.contains("QUBIT_CONFIG_UNSET_DESERIALIZE_VAR_12345"));
        }
        other => panic!("Expected SubstitutionError, got {:?}", other),
    }
}

#[test]
fn test_deserialize_unresolved_variable_in_json_leaf_returns_substitution_error() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct ServiceConfig {
        meta: serde_json::Value,
    }

    let mut config = Config::new();
    config
        .insert_property(
            "svc.meta",
            Property::new(
                "svc.meta",
                MultiValues::Json(vec![serde_json::json!({
                    "url": "${QUBIT_CONFIG_UNSET_JSON_LEAF_VAR_12345}/v1",
                })]),
            )
            .unwrap(),
        )
        .unwrap();

    let err = config
        .deserialize_interpolated::<ServiceConfig>("svc")
        .expect_err("unresolved JSON leaf variable should fail before serde deserialization");

    match err {
        ConfigError::SubstitutionError { path, message } => {
            assert_eq!(path, "svc.meta");
            assert!(message.contains("QUBIT_CONFIG_UNSET_JSON_LEAF_VAR_12345"));
        }
        other => panic!("Expected SubstitutionError, got {:?}", other),
    }
}
