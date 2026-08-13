// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # [`qubit_config::Config`] unit tests
//
// Covers the public `Config` API (including APIs introduced in v0.4.0).

#![allow(dead_code, unused_imports)]

#[cfg(feature = "chrono")]
pub(crate) use chrono::DateTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveDate;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveDateTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::Utc;
pub(crate) use qubit_config::Config;
pub(crate) use qubit_config::ConfigError;
pub(crate) use qubit_config::ConfigReader;
pub(crate) use qubit_config::Property;
pub(crate) use qubit_config::options::InterpolationSources;
pub(crate) use qubit_config::options::ReadPolicy;
pub(crate) use qubit_config::source::SourceLimitKind;
pub(crate) use qubit_config::source::SourceLimits;
pub(crate) use qubit_datatype::BlankStringPolicy;
pub(crate) use qubit_datatype::DataConversionError;
pub(crate) use qubit_datatype::DataConversionErrorKind;
pub(crate) use qubit_datatype::DataType;
pub(crate) use qubit_datatype::InvalidValueReason;
pub(crate) use qubit_value::MultiValues;
pub(crate) use qubit_value::ValueError;
pub(crate) use serde::Deserialize;

/// Creates a test configuration object
pub(crate) fn create_test_config() -> Config {
    let mut config = Config::new();
    config.set("string_value", "test").unwrap();
    config.set("int_value", 42).unwrap();
    config.set("bool_value", true).unwrap();
    config.set("float_value", 3.5).unwrap();
    config
}

/// Creates a test configuration with description
#[allow(dead_code)]
pub(crate) fn create_test_config_with_description() -> Config {
    Config::with_description("Test Configuration")
}

/// Changes the interpolation recursion limit while preserving other options.
pub(crate) fn set_max_interpolation_depth(
    config: &mut Config,
    max_depth: usize,
) {
    let options = config
        .default_read_policy()
        .clone()
        .with_max_interpolation_depth(max_depth);
    config.set_default_read_policy(options);
}
// ============================================================================
// Enhanced Error Model Tests
// ============================================================================

#[cfg(test)]
mod test_enhanced_errors {
    use super::Config;
    use super::ConfigError;
    use super::DataConversionError;
    use super::DataConversionErrorKind;
    use super::DataType;
    use super::Deserialize;
    use super::InvalidValueReason;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_type_mismatch_carries_key() {
        let mut config = Config::new();
        config.set("server.port", 8080).unwrap();

        let result: Result<bool, _> = config.get_strict("server.port");
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

        let result: Result<Vec<bool>, _> = config.get_list_strict("ports");
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
        let result: Result<String, _> =
            config.get("http.logging.body_size_limit");
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
        config.set_null("empty.key", DataType::String).unwrap();
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
            source_index: None,
            source: DataConversionError::invalid(
                DataType::String,
                DataType::Duration,
                InvalidValueReason::InvalidSyntax {
                    expected: "a duration",
                },
            ),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("db.timeout"));
        assert!(msg.contains("invalid syntax"));
    }

    #[test]
    fn test_deserialize_error_format_includes_path() {
        let error = ConfigError::DeserializeError {
            path: "http.server".to_string(),
            message: "missing field `port`".to_string(),
            source: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("http.server"));
        assert!(msg.contains("missing field"));
    }

    #[test]
    fn test_type_mismatch_from_value_error_requires_explicit_key() {
        use qubit_value::ValueError;
        let ve = ValueError::TypeMismatch {
            expected: DataType::Int32,
            actual: DataType::String,
        };
        let ce = ConfigError::from(("typed.value", ve));
        match ce {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "typed.value");
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
        let result: Result<bool, _> = config.get_strict("my.key");
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "my.key");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[test]
    fn test_conversion_error_from_value_error_requires_explicit_key() {
        use qubit_value::ValueError;
        let ve = ValueError::Conversion(DataConversionError::invalid(
            DataType::String,
            DataType::Int32,
            InvalidValueReason::OutOfRange,
        ));
        let ce = ConfigError::from(("converted.value", ve));
        match ce {
            ConfigError::ConversionError { key, source, .. } => {
                assert_eq!(key, "converted.value");
                assert_eq!(
                    source.kind(),
                    DataConversionErrorKind::InvalidValue,
                );
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_conversion_failed_from_value_error_requires_explicit_key() {
        use qubit_value::ValueError;
        let ve = ValueError::Conversion(DataConversionError::unsupported(
            DataType::String,
            DataType::Int32,
        ));
        let ce = ConfigError::from(("unsupported.value", ve));
        match ce {
            ConfigError::ConversionError { key, source, .. } => {
                assert_eq!(key, "unsupported.value");
                assert_eq!(source.kind(), DataConversionErrorKind::Unsupported,);
            }
            _ => panic!("Expected ConversionError"),
        }
    }
}

// ============================================================================
// TOML Type-Faithful Loading Tests
// ============================================================================

#[cfg(all(test, feature = "toml"))]
mod test_toml_type_faithful {
    use qubit_config::source::ConfigSource;
    use qubit_config::source::SourceLimitKind;
    use qubit_config::source::SourceLimits;
    use qubit_config::source::TomlConfigSource;

    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    fn load_toml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(&path, content).unwrap();
        let source = TomlConfigSource::from_file(&path);
        source.load().unwrap()
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
        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
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
        assert_eq!(config.get::<String>("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_toml_mixed_array_returns_source_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.toml");
        std::fs::write(&path, "mixed = [1, \"two\", 3]\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let error = source
            .load()
            .expect_err("mixed TOML arrays should be rejected");
        assert!(matches!(
            error,
            ConfigError::SourceParseError {
                path: Some(path),
                source_index: Some(1),
                ..
            } if path == "mixed"
        ));
    }

    #[test]
    fn test_toml_nested_array_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_array.toml");
        std::fs::write(&path, "nested = [[1, 2], [3, 4]]\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let result = source.load();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ConfigError::SourceParseError {
                path: Some(path),
                source_index: Some(0),
                ..
            }) if path == "nested"
        ));
    }

    #[test]
    fn test_toml_scalar_array_elements_count_toward_node_budget() {
        let source = TomlConfigSource::from_content("ports = [1, 2]\n")
            .with_limits(SourceLimits::default().with_max_nodes(3));
        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded { kind, .. })
                if kind == SourceLimitKind::NodeCount
        ));
    }
}

// ============================================================================
// YAML Type-Faithful Loading Tests
// ============================================================================

#[cfg(all(test, feature = "yaml"))]
mod test_yaml_type_faithful {
    use qubit_config::source::ConfigSource;
    use qubit_config::source::SourceLimitKind;
    use qubit_config::source::SourceLimits;
    use qubit_config::source::YamlConfigSource;

    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    fn load_yaml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(&path, content).unwrap();
        let source = YamlConfigSource::from_file(&path);
        source.load().unwrap()
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
        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
    }

    #[test]
    fn test_yaml_null_stored_as_empty_property() {
        let config = load_yaml("key: ~\n");
        assert!(config.contains("key").unwrap());
        assert!(config.is_unset("key").unwrap());
    }

    #[test]
    fn test_yaml_null_keyword() {
        let config = load_yaml("key: null\n");
        assert!(config.contains("key").unwrap());
        assert!(config.is_unset("key").unwrap());
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
        assert_eq!(config.get::<String>("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_yaml_mixed_sequence_returns_source_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.yaml");
        std::fs::write(&path, "mixed:\n  - 1\n  - two\n  - 3\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let error = source
            .load()
            .expect_err("mixed YAML sequences should be rejected");
        assert!(matches!(
            error,
            ConfigError::SourceParseError {
                path: Some(path),
                source_index: Some(1),
                ..
            } if path == "mixed"
        ));
    }

    #[test]
    fn test_yaml_tagged_value() {
        // Tagged values should be unwrapped
        let config = load_yaml("key: !!str 42\n");
        // The YAML backend treats !!str 42 as a string.
        assert!(config.contains("key").unwrap());
    }

    #[test]
    fn test_yaml_empty_sequence() {
        let config = load_yaml("empty: []\n");
        assert!(config.contains("empty").unwrap());
        assert_eq!(
            config.get_list::<String>("empty").unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(config.get_list::<i64>("empty").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_yaml_nested_sequence_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_seq.yaml");
        std::fs::write(&path, "matrix:\n  - [1, 2]\n  - [3, 4]\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let result = source.load();
        assert!(matches!(
            result,
            Err(ConfigError::SourceParseError {
                path: Some(path),
                source_index: Some(0),
                ..
            }) if path == "matrix"
        ));
    }

    #[test]
    fn test_yaml_scalar_sequence_elements_count_toward_node_budget() {
        let source = YamlConfigSource::from_content("ports: [1, 2]\n")
            .with_limits(SourceLimits::default().with_max_nodes(3));
        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded { kind, .. })
                if kind == SourceLimitKind::NodeCount
        ));
    }
}

// ============================================================================
// insert_property() / set_null() Tests
// ============================================================================

#[cfg(test)]
mod test_property_insertion_api {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_insert_property_success() {
        let mut config = Config::new();
        config
            .insert_property(
                "direct",
                Property::new(
                    "direct",
                    MultiValues::String(vec!["hello".to_string()]),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(config.get::<String>("direct").unwrap(), "hello");
    }

    #[test]
    fn test_set_null_success() {
        let mut config = Config::new();
        config.set_null("null_key", DataType::String).unwrap();
        assert!(config.is_unset("null_key").unwrap());
        assert!(config.contains("null_key").unwrap());
    }

    #[test]
    fn test_insert_property_name_mismatch_returns_error() {
        let mut config = Config::new();
        let property = Property::new(
            "actual.key",
            MultiValues::String(vec!["hello".to_string()]),
        )
        .unwrap();
        let result = config.insert_property("expected.key", property);
        assert!(matches!(result, Err(ConfigError::MergeError(_))));
    }

    #[test]
    fn test_insert_property_on_final_key_returns_error() {
        let mut config = Config::new();
        config.set("final.key", "v1").unwrap();
        config.set_final("final.key", true).unwrap();

        let result = config.insert_property(
            "final.key",
            Property::new(
                "final.key",
                MultiValues::String(vec!["v2".to_string()]),
            )
            .unwrap(),
        );
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
    }
}

// ============================================================================
// Additional behavior checks for config.rs error branches
// ============================================================================

#[cfg(test)]
mod test_config_error_branches {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_list_on_unset_property_reports_no_value() {
        let mut config = Config::new();
        config.set_null("empty", DataType::Int32).unwrap();
        let result = config.get_list::<i32>("empty");

        assert!(matches!(
            result,
            Err(ConfigError::PropertyHasNoValue(key)) if key == "empty"
        ));
    }

    // Test get on a property that has wrong type (triggers TypeMismatch with
    // key)
    #[test]
    fn test_get_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("http.port", 8080).unwrap();
        let err = config.get_strict::<String>("http.port").unwrap_err();
        match err {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "http.port");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    // Test get_list on a property that has wrong type (triggers TypeMismatch
    // with key)
    #[test]
    fn test_get_list_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("ports", vec![8080i32, 8081]).unwrap();
        let err = config.get_list_strict::<String>("ports").unwrap_err();
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
        config.set_null("empty_str", DataType::String).unwrap();
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
// merge_properties_from_source (`Config` API)
// ============================================================================

#[cfg(all(test, feature = "toml"))]
mod test_merge_properties_from_source {
    use std::path::PathBuf;

    use qubit_config::source::TomlConfigSource;

    use super::Config;
    use super::ConfigError;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn test_merge_properties_from_source_populates_config() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert!(!config.is_empty());
        assert!(config.contains("host").unwrap());
    }

    #[test]
    fn test_merge_properties_from_source_overwrites_existing_keys() {
        let mut config = Config::new();
        config.set("host", "old-host").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_properties_from_source(&source).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
    }

    #[test]
    fn test_merge_properties_from_source_preserves_final_property() {
        let mut config = Config::new();
        config.set("host", "final-host").unwrap();
        config.set_final("host", true).unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let result = config.merge_properties_from_source(&source);

        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get::<String>("host").unwrap(), "final-host");
    }

    #[test]
    fn test_merge_properties_from_source_adds_new_keys() {
        let mut config = Config::new();
        config.set("existing", "value").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_properties_from_source(&source).unwrap();

        assert_eq!(config.get::<String>("existing").unwrap(), "value");
        assert!(config.contains("host").unwrap());
        assert!(config.contains("app.name").unwrap());
    }

    #[test]
    fn test_merge_properties_from_source_returns_error_on_failure() {
        let source = TomlConfigSource::from_file("/nonexistent/path.toml");
        let mut config = Config::new();
        let result = config.merge_properties_from_source(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_properties_from_source_with_variable_substitution() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vars.toml");
        std::fs::write(
            &path,
            r#"
base_url = "http://localhost:8080"
api_url = "${base_url}/api"
"#,
        )
        .unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert_eq!(
            config.get_interpolated::<String>("api_url").unwrap(),
            "http://localhost:8080/api"
        );
    }
}

// ============================================================================
// Source-backed constructors (`Config` API)
// ============================================================================

#[cfg(all(test, feature = "env-file", feature = "toml", feature = "yaml"))]
mod test_source_backed_constructors {
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::sync::OnceLock;

    use qubit_config::source::EnvConfigOptions;
    use qubit_config::source::TomlConfigSource;

    use super::Config;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    /// Serializes constructor tests that mutate process environment variables.
    fn env_test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("environment test lock should not be poisoned")
    }

    #[test]
    fn test_from_source_loads_config_source_into_new_config() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));

        let config = Config::from_source(&source).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_from_toml_file_loads_toml_config() {
        let config = Config::from_toml_file(fixture("basic.toml")).unwrap();

        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_from_yaml_file_loads_yaml_config() {
        let config = Config::from_yaml_file(fixture("basic.yaml")).unwrap();

        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_from_properties_file_loads_properties_config() {
        let config =
            Config::from_properties_file(fixture("basic.properties")).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
        assert_eq!(config.get::<String>("app.version").unwrap(), "1.0.0");
    }

    #[test]
    fn test_from_env_file_loads_dotenv_config() {
        let config = Config::from_env_file(fixture("basic.env")).unwrap();

        assert_eq!(config.get::<String>("HOST").unwrap(), "localhost");
        assert_eq!(config.get::<String>("APP_NAME").unwrap(), "MyApp");
    }

    #[test]
    fn test_from_env_loads_process_environment() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QUBIT_CONFIG_FROM_ENV_TEST_KEY", "from-env");
        }

        let config = Config::from_env().unwrap();

        assert_eq!(
            config
                .get::<String>("QUBIT_CONFIG_FROM_ENV_TEST_KEY")
                .unwrap(),
            "from-env"
        );

        unsafe {
            std::env::remove_var("QUBIT_CONFIG_FROM_ENV_TEST_KEY");
        }
    }

    #[test]
    fn test_from_env_prefix_loads_and_normalizes_matching_vars() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QCFG_SERVER__HOST", "env-host");
            std::env::set_var("QCFG_SERVER__PORT", "9091");
            std::env::set_var("OTHER_QCFG_SERVER__HOST", "ignored");
        }

        let config = Config::from_env_prefix("QCFG_").unwrap();

        assert_eq!(config.get::<String>("server.host").unwrap(), "env-host");
        assert_eq!(config.get::<String>("server.port").unwrap(), "9091");
        assert!(!config.contains("OTHER_QCFG_SERVER__HOST").unwrap());

        unsafe {
            std::env::remove_var("QCFG_SERVER__HOST");
            std::env::remove_var("QCFG_SERVER__PORT");
            std::env::remove_var("OTHER_QCFG_SERVER__HOST");
        }
    }

    #[test]
    fn test_from_env_options_respects_explicit_key_transform_options() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QOPTS_MY_KEY", "raw-value");
        }

        let config =
            Config::from_env_options(EnvConfigOptions::new().prefix("QOPTS_"))
                .unwrap();

        assert_eq!(config.get::<String>("QOPTS_MY_KEY").unwrap(), "raw-value");
        assert!(!config.contains("my.key").unwrap());

        unsafe {
            std::env::remove_var("QOPTS_MY_KEY");
        }
    }

    #[test]
    fn test_from_toml_file_returns_error_for_missing_file() {
        let result = Config::from_toml_file("/nonexistent/path.toml");

        assert!(result.is_err());
    }
}
