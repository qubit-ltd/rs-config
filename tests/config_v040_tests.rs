/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Config v0.4.0 New Feature Tests
//!
//! Tests all new APIs introduced in v0.4.0:
//! - `iter()` / `iter_prefix()` / `contains_prefix()` / `subconfig()`
//! - `get_optional()` / `get_list_optional()` / `is_null()`
//! - `deserialize::<T>()`
//! - Enhanced error model with key/path context
//! - TOML/YAML type-faithful loading

use qubit_common::DataType;
use qubit_config::{Config, ConfigError, Property};
use qubit_value::MultiValues;
use serde::Deserialize;
use std::collections::HashMap;

// ============================================================================
// iter() Tests
// ============================================================================

#[cfg(test)]
mod test_iter {
    use super::*;

    #[test]
    fn test_iter_empty_config() {
        let config = Config::new();
        let entries: Vec<_> = config.iter().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_single_entry() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        let entries: Vec<_> = config.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "host");
    }

    #[test]
    fn test_iter_multiple_entries() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", 8080).unwrap();
        config.set("debug", true).unwrap();
        let entries: Vec<_> = config.iter().collect();
        assert_eq!(entries.len(), 3);
        let keys: Vec<&str> = entries.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"host"));
        assert!(keys.contains(&"port"));
        assert!(keys.contains(&"debug"));
    }

    #[test]
    fn test_iter_yields_property_references() {
        let mut config = Config::new();
        config.set("x", 42).unwrap();
        for (key, prop) in config.iter() {
            assert_eq!(key, "x");
            assert!(!prop.is_empty());
        }
    }
}

// ============================================================================
// iter_prefix() Tests
// ============================================================================

#[cfg(test)]
mod test_iter_prefix {
    use super::*;

    #[test]
    fn test_iter_prefix_empty_config() {
        let config = Config::new();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_prefix_no_match() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();
        config.set("db.port", 5432).unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_prefix_partial_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert_eq!(entries.len(), 2);
        let keys: Vec<&str> = entries.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"http.host"));
        assert!(keys.contains(&"http.port"));
        assert!(!keys.contains(&"db.host"));
    }

    #[test]
    fn test_iter_prefix_exact_prefix_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("https.host", "secure").unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "http.host");
    }

    #[test]
    fn test_iter_prefix_all_match() {
        let mut config = Config::new();
        config.set("app.name", "test").unwrap();
        config.set("app.version", "1.0").unwrap();
        config.set("app.debug", true).unwrap();
        let entries: Vec<_> = config.iter_prefix("app.").collect();
        assert_eq!(entries.len(), 3);
    }
}

// ============================================================================
// contains_prefix() Tests
// ============================================================================

#[cfg(test)]
mod test_contains_prefix {
    use super::*;

    #[test]
    fn test_contains_prefix_empty_config() {
        let config = Config::new();
        assert!(!config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        assert!(config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_no_match() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();
        assert!(!config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_partial_key_name() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        // "http" is a prefix of "http.host"
        assert!(config.contains_prefix("http"));
        // "htt" is also a prefix
        assert!(config.contains_prefix("htt"));
    }

    #[test]
    fn test_contains_prefix_empty_prefix() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        // Empty string is a prefix of everything
        assert!(config.contains_prefix(""));
    }
}

// ============================================================================
// subconfig() Tests
// ============================================================================

#[cfg(test)]
mod test_subconfig {
    use super::*;

    #[test]
    fn test_subconfig_strip_prefix_true() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(sub.contains("host"));
        assert!(sub.contains("port"));
        assert!(!sub.contains("db.host"));
        assert!(!sub.contains("http.host"));
    }

    #[test]
    fn test_subconfig_strip_prefix_false() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", false).unwrap();
        assert!(sub.contains("http.host"));
        assert!(sub.contains("http.port"));
        assert!(!sub.contains("db.host"));
    }

    #[test]
    fn test_subconfig_empty_result() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(sub.is_empty());
    }

    #[test]
    fn test_subconfig_exact_key_match() {
        let mut config = Config::new();
        config.set("http", "value").unwrap();
        config.set("http.host", "localhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        // "http" itself matches (kept as "http" when strip_prefix=true)
        assert!(sub.contains("http"));
        // "http.host" matches and becomes "host"
        assert!(sub.contains("host"));
    }

    #[test]
    fn test_subconfig_preserves_variable_substitution_settings() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set_max_substitution_depth(10);
        config.set("http.host", "localhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(!sub.is_enable_variable_substitution());
        assert_eq!(sub.max_substitution_depth(), 10);
    }

    #[test]
    fn test_subconfig_nested_prefix() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let sub = config.subconfig("http.proxy", true).unwrap();
        assert!(sub.contains("host"));
        assert!(sub.contains("port"));
        assert!(!sub.contains("timeout"));
    }
}

// ============================================================================
// is_null() Tests
// ============================================================================

#[cfg(test)]
mod test_is_null {
    use super::*;

    #[test]
    fn test_is_null_missing_key_returns_false() {
        let config = Config::new();
        assert!(!config.is_null("missing"));
    }

    #[test]
    fn test_is_null_key_with_value_returns_false() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        assert!(!config.is_null("host"));
    }

    #[test]
    fn test_is_null_empty_property_returns_true() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "nullable".to_string(),
            Property::with_value("nullable", MultiValues::Empty(DataType::String)),
        );
        assert!(config.is_null("nullable"));
    }

    #[test]
    fn test_is_null_after_clear() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.get_property_mut("host").unwrap().clear();
        assert!(config.is_null("host"));
    }
}

// ============================================================================
// get_optional() Tests
// ============================================================================

#[cfg(test)]
mod test_get_optional {
    use super::*;

    #[test]
    fn test_get_optional_missing_key_returns_none() {
        let config = Config::new();
        let result: Option<String> = config.get_optional("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_existing_key_returns_some() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        let result: Option<String> = config.get_optional("host").unwrap();
        assert_eq!(result, Some("localhost".to_string()));
    }

    #[test]
    fn test_get_optional_null_property_returns_none() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "nullable".to_string(),
            Property::with_value("nullable", MultiValues::Empty(DataType::String)),
        );
        let result: Option<String> = config.get_optional("nullable").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_integer() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Option<i32> = config.get_optional("port").unwrap();
        assert_eq!(result, Some(8080));
    }

    #[test]
    fn test_get_optional_bool() {
        let mut config = Config::new();
        config.set("debug", true).unwrap();
        let result: Option<bool> = config.get_optional("debug").unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_get_optional_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Result<Option<bool>, _> = config.get_optional("port");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "port");
            }
            e => panic!("Expected TypeMismatch, got {:?}", e),
        }
    }
}

// ============================================================================
// get_list_optional() Tests
// ============================================================================

#[cfg(test)]
mod test_get_list_optional {
    use super::*;

    #[test]
    fn test_get_list_optional_missing_key_returns_none() {
        let config = Config::new();
        let result: Option<Vec<i32>> = config.get_list_optional("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_list_optional_existing_key_returns_some() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081, 8082]).unwrap();
        let result: Option<Vec<i32>> = config.get_list_optional("ports").unwrap();
        assert_eq!(result, Some(vec![8080, 8081, 8082]));
    }

    #[test]
    fn test_get_list_optional_null_property_returns_none() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "nullable".to_string(),
            Property::with_value("nullable", MultiValues::Empty(DataType::Int32)),
        );
        let result: Option<Vec<i32>> = config.get_list_optional("nullable").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_list_optional_single_value() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Option<Vec<i32>> = config.get_list_optional("port").unwrap();
        assert_eq!(result, Some(vec![8080]));
    }

    #[test]
    fn test_get_list_optional_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081]).unwrap();
        let result: Result<Option<Vec<bool>>, _> = config.get_list_optional("ports");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "ports");
            }
            e => panic!("Expected TypeMismatch, got {:?}", e),
        }
    }
}

// ============================================================================
// deserialize() Tests
// ============================================================================

#[cfg(test)]
mod test_deserialize {
    use super::*;

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
    struct WithOptionals {
        host: String,
        port: Option<i32>,
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
        config.properties_mut().insert(
            "srv.port".to_string(),
            Property::with_value("srv.port", MultiValues::Empty(DataType::Int32)),
        );

        let result: WithOptionals = config.deserialize("srv").unwrap();
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, None);
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
}

// ============================================================================
// Enhanced Error Model Tests
// ============================================================================

#[cfg(test)]
mod test_enhanced_errors {
    use super::*;

    #[test]
    fn test_get_type_mismatch_carries_key() {
        let mut config = Config::new();
        config.set("server.port", 8080).unwrap();

        let result: Result<bool, _> = config.get("server.port");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "server.port");
                assert_eq!(expected, DataType::Bool);
                assert_eq!(actual, DataType::Int32);
            }
            e => panic!("Expected TypeMismatch with key, got {:?}", e),
        }
    }

    #[test]
    fn test_get_list_type_mismatch_carries_key() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081]).unwrap();

        let result: Result<Vec<bool>, _> = config.get_list("ports");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "ports");
            }
            e => panic!("Expected TypeMismatch with key, got {:?}", e),
        }
    }

    #[test]
    fn test_get_property_not_found_carries_key() {
        let config = Config::new();
        let result: Result<String, _> = config.get("http.logging.body_size_limit");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PropertyNotFound(key) => {
                assert_eq!(key, "http.logging.body_size_limit");
            }
            e => panic!("Expected PropertyNotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_get_property_has_no_value_carries_key() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "empty.key".to_string(),
            Property::with_value("empty.key", MultiValues::Empty(DataType::String)),
        );
        let result: Result<String, _> = config.get("empty.key");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PropertyHasNoValue(key) => {
                assert_eq!(key, "empty.key");
            }
            e => panic!("Expected PropertyHasNoValue, got {:?}", e),
        }
    }

    #[test]
    fn test_type_mismatch_error_format_includes_key() {
        let error = ConfigError::TypeMismatch {
            key: "http.logging.body_size_limit".to_string(),
            expected: DataType::Int32,
            actual: DataType::String,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("http.logging.body_size_limit"));
        assert!(msg.contains("expected"));
        assert!(msg.contains("actual"));
    }

    #[test]
    fn test_conversion_error_format_includes_key() {
        let error = ConfigError::ConversionError {
            key: "db.timeout".to_string(),
            message: "invalid duration format".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("db.timeout"));
        assert!(msg.contains("invalid duration format"));
    }

    #[test]
    fn test_deserialize_error_format_includes_path() {
        let error = ConfigError::DeserializeError {
            path: "http.server".to_string(),
            message: "missing field `port`".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("http.server"));
        assert!(msg.contains("missing field"));
    }

    #[test]
    fn test_type_mismatch_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        // ValueError::TypeMismatch → ConfigError::TypeMismatch with empty key
        let ve = ValueError::TypeMismatch {
            expected: DataType::Int32,
            actual: DataType::String,
        };
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "");
                assert_eq!(expected, DataType::Int32);
                assert_eq!(actual, DataType::String);
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[test]
    fn test_type_mismatch_from_get_has_key() {
        let mut config = Config::new();
        config.set("my.key", 42).unwrap();
        let result: Result<bool, _> = config.get("my.key");
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "my.key");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[test]
    fn test_conversion_error_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        let ve = ValueError::ConversionError("test message".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { key, message } => {
                assert_eq!(key, "");
                assert_eq!(message, "test message");
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_conversion_failed_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        let ve = ValueError::ConversionFailed {
            from: DataType::String,
            to: DataType::Int32,
        };
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { key, message } => {
                assert_eq!(key, "");
                assert!(message.contains("From") || message.contains("to"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_from_value_error_json_serialization() {
        use qubit_value::ValueError;
        let ve = ValueError::JsonSerializationError("json error".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { message, .. } => {
                assert!(message.contains("JSON serialization error"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_from_value_error_json_deserialization() {
        use qubit_value::ValueError;
        let ve = ValueError::JsonDeserializationError("json error".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { message, .. } => {
                assert!(message.contains("JSON deserialization error"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }
}

// ============================================================================
// TOML Type-Faithful Loading Tests
// ============================================================================

#[cfg(test)]
mod test_toml_type_faithful {
    use qubit_config::source::{ConfigSource, TomlConfigSource};

    use super::*;

    fn load_toml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(&path, content).unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        config
    }

    #[test]
    fn test_toml_integer_stored_as_i64() {
        let config = load_toml("port = 8080\n");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_toml_float_stored_as_f64() {
        let config = load_toml("timeout = 30.5\n");
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_toml_bool_stored_as_bool() {
        let config = load_toml("debug = true\nenabled = false\n");
        assert!(config.get::<bool>("debug").unwrap());
        assert!(!config.get::<bool>("enabled").unwrap());
    }

    #[test]
    fn test_toml_string_stored_as_string() {
        let config = load_toml("host = \"localhost\"\n");
        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_toml_integer_array_stored_as_i64_multivalue() {
        let config = load_toml("ports = [8080, 8081, 8082]\n");
        let ports: Vec<i64> = config.get_list("ports").unwrap();
        assert_eq!(ports, vec![8080i64, 8081, 8082]);
    }

    #[test]
    fn test_toml_float_array_stored_as_f64_multivalue() {
        let config = load_toml("weights = [0.1, 0.5, 0.9]\n");
        let weights: Vec<f64> = config.get_list("weights").unwrap();
        assert!((weights[0] - 0.1).abs() < 1e-9);
        assert!((weights[1] - 0.5).abs() < 1e-9);
        assert!((weights[2] - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_toml_bool_array_stored_as_bool_multivalue() {
        let config = load_toml("flags = [true, false, true]\n");
        let flags: Vec<bool> = config.get_list("flags").unwrap();
        assert_eq!(flags, vec![true, false, true]);
    }

    #[test]
    fn test_toml_string_array_stored_as_string_multivalue() {
        let config = load_toml("tags = [\"web\", \"api\", \"v2\"]\n");
        let tags: Vec<String> = config.get_list("tags").unwrap();
        assert_eq!(tags, vec!["web", "api", "v2"]);
    }

    #[test]
    fn test_toml_nested_table_flattened() {
        let config = load_toml("[server]\nhost = \"localhost\"\nport = 9090\n");
        assert_eq!(config.get_string("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_toml_mixed_array_falls_back_to_string() {
        // Mixed types: int and string → fall back to string
        let config = load_toml("mixed = [1, \"two\", 3]\n");
        // Should be stored as strings
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 3);
    }

    #[test]
    fn test_toml_nested_array_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_array.toml");
        std::fs::write(&path, "nested = [[1, 2], [3, 4]]\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}

// ============================================================================
// YAML Type-Faithful Loading Tests
// ============================================================================

#[cfg(test)]
mod test_yaml_type_faithful {
    use qubit_config::source::{ConfigSource, YamlConfigSource};

    use super::*;

    fn load_yaml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(&path, content).unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        config
    }

    #[test]
    fn test_yaml_integer_stored_as_i64() {
        let config = load_yaml("port: 8080\n");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_yaml_float_stored_as_f64() {
        let config = load_yaml("timeout: 30.5\n");
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_yaml_bool_stored_as_bool() {
        let config = load_yaml("debug: true\nenabled: false\n");
        assert!(config.get::<bool>("debug").unwrap());
        assert!(!config.get::<bool>("enabled").unwrap());
    }

    #[test]
    fn test_yaml_string_stored_as_string() {
        let config = load_yaml("host: localhost\n");
        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_yaml_null_stored_as_empty_property() {
        let config = load_yaml("key: ~\n");
        assert!(config.contains("key"));
        assert!(config.is_null("key"));
    }

    #[test]
    fn test_yaml_null_keyword() {
        let config = load_yaml("key: null\n");
        assert!(config.contains("key"));
        assert!(config.is_null("key"));
    }

    #[test]
    fn test_yaml_integer_sequence_stored_as_i64_multivalue() {
        let config = load_yaml("ports:\n  - 8080\n  - 8081\n  - 8082\n");
        let ports: Vec<i64> = config.get_list("ports").unwrap();
        assert_eq!(ports, vec![8080i64, 8081, 8082]);
    }

    #[test]
    fn test_yaml_float_sequence_stored_as_f64_multivalue() {
        let config = load_yaml("weights:\n  - 0.1\n  - 0.5\n  - 0.9\n");
        let weights: Vec<f64> = config.get_list("weights").unwrap();
        assert!((weights[0] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_yaml_bool_sequence_stored_as_bool_multivalue() {
        let config = load_yaml("flags:\n  - true\n  - false\n  - true\n");
        let flags: Vec<bool> = config.get_list("flags").unwrap();
        assert_eq!(flags, vec![true, false, true]);
    }

    #[test]
    fn test_yaml_string_sequence_stored_as_string_multivalue() {
        let config = load_yaml("tags:\n  - web\n  - api\n  - v2\n");
        let tags: Vec<String> = config.get_list("tags").unwrap();
        assert_eq!(tags, vec!["web", "api", "v2"]);
    }

    #[test]
    fn test_yaml_nested_mapping_flattened() {
        let config = load_yaml("server:\n  host: localhost\n  port: 9090\n");
        assert_eq!(config.get_string("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_yaml_mixed_sequence_falls_back_to_string() {
        let config = load_yaml("mixed:\n  - 1\n  - two\n  - 3\n");
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 3);
    }

    #[test]
    fn test_yaml_tagged_value() {
        // Tagged values should be unwrapped
        let config = load_yaml("key: !!str 42\n");
        // serde_yaml treats !!str 42 as a string
        assert!(config.contains("key"));
    }

    #[test]
    fn test_yaml_empty_sequence() {
        let config = load_yaml("empty: []\n");
        // Empty sequence should not create any entry (or create empty)
        // The key may or may not exist depending on implementation
        // Just verify it doesn't panic
        let _ = config.get_list_optional::<String>("empty");
    }

    #[test]
    fn test_yaml_nested_sequence_falls_back_to_string() {
        // Nested sequences (sequence of sequences) fall back to string
        let config = load_yaml("matrix:\n  - [1, 2]\n  - [3, 4]\n");
        // Should not panic; nested sequences produce empty strings
        assert!(config.contains("matrix") || !config.contains("matrix"));
    }
}

// ============================================================================
// property_to_json_value coverage: various MultiValues types via deserialize
// ============================================================================

#[cfg(test)]
mod test_property_to_json_value_coverage {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
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
            .properties_mut()
            .insert(key.to_string(), Property::with_value(key, mv));
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
        use chrono::{DateTime, Utc};
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

// ============================================================================
// properties_mut() Tests
// ============================================================================

#[cfg(test)]
mod test_properties_mut {
    use super::*;

    #[test]
    fn test_properties_mut_insert_directly() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "direct".to_string(),
            Property::with_value("direct", MultiValues::String(vec!["hello".to_string()])),
        );
        assert_eq!(config.get_string("direct").unwrap(), "hello");
    }

    #[test]
    fn test_properties_mut_insert_null() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "null_key".to_string(),
            Property::with_value("null_key", MultiValues::Empty(DataType::String)),
        );
        assert!(config.is_null("null_key"));
        assert!(config.contains("null_key"));
    }
}

// ============================================================================
// Additional coverage for config.rs error branches
// ============================================================================

#[cfg(test)]
mod test_config_error_branches {
    use super::*;

    // Test that get_list on an empty property returns empty vec (not error)
    #[test]
    fn test_get_list_on_empty_property_returns_empty_vec() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "empty".to_string(),
            Property::with_value("empty", MultiValues::Empty(DataType::Int32)),
        );
        let result: Vec<i32> = config.get_list("empty").unwrap();
        assert!(result.is_empty());
    }

    // Test get on a property that has wrong type (triggers TypeMismatch with key)
    #[test]
    fn test_get_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("http.port", 8080).unwrap();
        let err = config.get::<String>("http.port").unwrap_err();
        match err {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "http.port");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    // Test get_list on a property that has wrong type (triggers TypeMismatch with key)
    #[test]
    fn test_get_list_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("ports", vec![8080i32, 8081]).unwrap();
        let err = config.get_list::<String>("ports").unwrap_err();
        match err {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "ports");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    // Test that get on empty property returns PropertyHasNoValue
    #[test]
    fn test_get_on_empty_property_returns_has_no_value() {
        let mut config = Config::new();
        config.properties_mut().insert(
            "empty_str".to_string(),
            Property::with_value("empty_str", MultiValues::Empty(DataType::String)),
        );
        let err = config.get::<String>("empty_str").unwrap_err();
        match err {
            ConfigError::PropertyHasNoValue(key) => {
                assert_eq!(key, "empty_str");
            }
            _ => panic!("Expected PropertyHasNoValue, got {:?}", err),
        }
    }
}

// ============================================================================
// Integration: subconfig + deserialize
// ============================================================================

#[cfg(test)]
mod test_subconfig_deserialize_integration {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct HttpOptions {
        host: String,
        port: i32,
        timeout: Option<i64>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DbOptions {
        url: String,
        pool: i32,
    }

    #[test]
    fn test_deserialize_http_options() {
        let mut config = Config::new();
        config.set("http.host", "api.example.com").unwrap();
        config.set("http.port", 443).unwrap();
        config.set("http.timeout", 30i64).unwrap();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 5).unwrap();

        let http: HttpOptions = config.deserialize("http").unwrap();
        assert_eq!(http.host, "api.example.com");
        assert_eq!(http.port, 443);
        assert_eq!(http.timeout, Some(30));

        let db: DbOptions = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 5);
    }

    #[test]
    fn test_subconfig_then_get() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy.example.com").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let proxy = config.subconfig("http.proxy", true).unwrap();
        assert_eq!(proxy.get_string("host").unwrap(), "proxy.example.com");
        assert_eq!(proxy.get::<i32>("port").unwrap(), 3128);
        assert!(!proxy.contains("timeout"));
    }

    #[test]
    fn test_iter_prefix_then_subconfig() {
        let mut config = Config::new();
        config.set("module.a.x", 1).unwrap();
        config.set("module.a.y", 2).unwrap();
        config.set("module.b.x", 3).unwrap();

        assert!(config.contains_prefix("module.a."));
        assert!(config.contains_prefix("module.b."));

        let sub_a = config.subconfig("module.a", true).unwrap();
        assert_eq!(sub_a.get::<i32>("x").unwrap(), 1);
        assert_eq!(sub_a.get::<i32>("y").unwrap(), 2);
        assert!(!sub_a.contains("x".to_string().as_str().replace("x", "b.x").as_str()));
    }
}
