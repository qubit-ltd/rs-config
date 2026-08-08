// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg(feature = "yaml")]

// # `YamlConfigSource` tests

use std::path::PathBuf;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigResult;
use qubit_config::Property;
use qubit_config::source::ConfigSource;
use qubit_config::source::YamlConfigSource;
use qubit_value::MultiValues;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn merge_source(
    config: &mut Config,
    source: &dyn ConfigSource,
) -> ConfigResult<()> {
    config.merge_from_source(source)
}

// ============================================================================
// YamlConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_yaml_config_source {
    use super::Config;
    use super::ConfigError;
    use super::ConfigErrorKind;
    use super::ConfigSource;
    use super::MultiValues;
    use super::PathBuf;
    use super::Property;
    use super::YamlConfigSource;
    use super::fixture;
    use super::merge_source;

    #[test]
    fn test_load_basic_yaml_file() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // String values remain strings
        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
        // Boolean values are stored as bool
        assert!(config.get::<bool>("debug").unwrap());
        // Float values are stored as f64
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_load_yaml_u64_scalar_without_precision_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("u64_scalar.yaml");
        std::fs::write(&path, format!("value: {}\n", u64::MAX)).unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<u64>("value").unwrap(), u64::MAX);
    }

    #[test]
    fn test_load_yaml_u64_sequence_without_precision_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("u64_sequence.yaml");
        std::fs::write(
            &path,
            "values: [9223372036854775808, 18446744073709551615]\n",
        )
        .unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        merge_source(&mut config, &source).unwrap();

        assert_eq!(
            config.get_list::<u64>("values").unwrap(),
            vec![i64::MAX as u64 + 1, u64::MAX],
        );
    }

    #[test]
    fn test_load_yaml_nested_mapping_flattened() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<String>("app.version").unwrap(), "1.0.0");
        assert_eq!(config.get::<String>("server.host").unwrap(), "0.0.0.0");
        // Integer values are stored as i64
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_load_yaml_sequence_as_multivalue() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        let tags = config.get::<Vec<String>>("tags").unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"web".to_string()));
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"v2".to_string()));
    }

    #[test]
    fn test_load_nonexistent_yaml_file_returns_error() {
        let source =
            YamlConfigSource::from_file("/nonexistent/path/config.yaml");
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_load_invalid_yaml_returns_redacted_parse_error() {
        const SECRET_MARKER: &str = "RS_CONFIG_YAML_PARSER_SECRET_MARKER";
        let dir =
            tempfile::tempdir().expect("temporary directory should be created");
        let path = dir.path().join("invalid.yaml");
        std::fs::write(&path, format!("password: \"{SECRET_MARKER}\n"))
            .expect("invalid YAML fixture should be written");

        let source = YamlConfigSource::from_file(&path);
        let error = source
            .load()
            .expect_err("unterminated YAML string should fail");

        assert!(matches!(&error, ConfigError::SourceParseError { .. }));
        let display = error.to_string();
        assert!(display.contains("invalid YAML syntax"));
        assert!(display.contains("line"));
        assert!(display.contains("column"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!format!("{error:?}").contains(SECRET_MARKER));
    }

    #[test]
    fn test_merge_from_yaml_config_source() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(config.contains("host").unwrap());
        assert!(config.contains("app.name").unwrap());
    }

    #[test]
    fn test_load_yaml_rejects_aliases_before_materializing_values() {
        let source = YamlConfigSource::from_content(
            "base: &base\n  value: 1\ncopy: *base\n",
        );
        let error = source
            .load()
            .expect_err("YAML aliases should be rejected before flattening");

        assert_eq!(error.kind(), ConfigErrorKind::Parse);
        assert_eq!(error.source_id(), Some("YAML:<memory>"));
        assert!(error.to_string().contains("anchors and aliases"));
    }

    #[test]
    fn test_load_inline_yaml_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inline.yaml");
        std::fs::write(
            &path,
            r#"
name: test
value: 42
enabled: false
db:
  url: "postgres://localhost/mydb"
  pool: 5
"#,
        )
        .unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("name").unwrap(), "test");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("value").unwrap(), 42);
        // Boolean values are stored as bool
        assert!(!config.get::<bool>("enabled").unwrap());
        assert_eq!(
            config.get::<String>("db.url").unwrap(),
            "postgres://localhost/mydb"
        );
        // Integer values are stored as i64
        assert_eq!(config.get::<i64>("db.pool").unwrap(), 5);
    }

    #[test]
    fn test_load_yaml_null_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null.yaml");
        std::fs::write(&path, "key: ~\nother: value").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // Null values are preserved as empty properties (is_unset returns true)
        assert!(config.contains("key").unwrap());
        assert!(config.is_unset("key").unwrap());
        // get_optional returns None for null values
        let val: Option<String> = config.get_optional("key").unwrap();
        assert_eq!(val, None);
        assert_eq!(config.get::<String>("other").unwrap(), "value");
    }

    #[test]
    fn test_load_yaml_null_overrides_existing_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null_override.yaml");
        std::fs::write(&path, "key: null\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("key", "old").unwrap();

        merge_source(&mut config, &source).unwrap();

        assert!(config.contains("key").unwrap());
        assert!(config.is_unset("key").unwrap());
        let value: Option<String> = config.get_optional("key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_load_yaml_null_respects_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null_final.yaml");
        std::fs::write(&path, "locked: null\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let mut property = Property::new(
            "locked",
            MultiValues::String(vec!["old".to_string()]),
        )
        .unwrap();
        property.set_final(true);
        config.insert_property("locked", property).unwrap();

        let result = merge_source(&mut config, &source);

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get::<String>("locked").unwrap(), "old");
    }

    #[test]
    fn test_load_yaml_empty_sequence_is_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_sequence.yaml");
        std::fs::write(&path, "empty: []\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert!(config.contains("empty").unwrap());
        assert_eq!(
            config.get::<Vec<String>>("empty").unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(config.get_list::<i64>("empty").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_load_yaml_empty_sequence_overrides_existing_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_override.yaml");
        std::fs::write(&path, "ports: []\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("ports", vec![8080i64, 8081]).unwrap();

        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get_list::<i64>("ports").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_load_yaml_non_empty_sequence_overrides_existing_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sequence_override.yaml");
        std::fs::write(&path, "ports:\n  - 9000\n  - 9001\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("ports", vec![8080i64, 8081]).unwrap();

        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get_list::<i64>("ports").unwrap(), vec![9000, 9001]);
    }

    #[test]
    fn test_load_yaml_empty_sequence_deserializes_as_empty_vec() {
        #[derive(serde::Deserialize)]
        struct Service {
            ports: Vec<i64>,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_deserialize.yaml");
        std::fs::write(&path, "service:\n  ports: []\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        merge_source(&mut config, &source).unwrap();
        let service: Service = config.deserialize("service").unwrap();

        assert_eq!(service.ports, Vec::<i64>::new());
    }

    #[test]
    fn test_load_yaml_empty_sequence_respects_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_final.yaml");
        std::fs::write(&path, "locked: []\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("locked", vec!["old"]).unwrap();
        config.set_final("locked", true).unwrap();

        let result = merge_source(&mut config, &source);

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get::<Vec<String>>("locked").unwrap(), vec!["old"]);
    }

    #[test]
    fn test_load_yaml_complex_keys_returns_redacted_error() {
        const SECRET_MARKER: &str = "RS_CONFIG_YAML_KEY_SECRET_MARKER";
        let dir =
            tempfile::tempdir().expect("temporary directory should be created");
        let path = dir.path().join("complex_key.yaml");
        std::fs::write(&path, format!("? [{SECRET_MARKER}, b]\n: 1\n"))
            .expect("complex YAML key fixture should be written");

        let source = YamlConfigSource::from_file(&path);
        let error = source.load().expect_err("complex YAML key should fail");

        let display = error.to_string();
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!format!("{error:?}").contains(SECRET_MARKER));
    }

    #[test]
    fn test_from_file_clone_keeps_debug_path() {
        let path = PathBuf::from("config.yaml");
        let source = YamlConfigSource::from_file(&path);
        let cloned = source.clone();

        assert_eq!(format!("{source:?}"), format!("{cloned:?}"));
        assert!(format!("{source:?}").contains("config.yaml"));
    }

    #[test]
    fn test_load_yaml_rejects_flattened_key_collision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flatten_collision.yaml");
        std::fs::write(
            &path,
            r#"
"server.port": 8080
server:
  port: 9090
"#,
        )
        .unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("existing", "old").unwrap();
        let result = merge_source(&mut config, &source);

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "server.port"
        ));
        assert_eq!(config.get::<String>("existing").unwrap(), "old");
        assert_eq!(config.len(), 1);
    }

    #[test]
    fn test_load_yaml_rejects_coerced_key_collision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("coerced_collision.yaml");
        std::fs::write(
            &path,
            r#"
true: enabled
"true": string
"#,
        )
        .unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path == "true"
        ));
        assert!(config.is_empty());
    }
}

#[cfg(test)]
mod test_yaml_edge_cases {
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::YamlConfigSource;
    use super::merge_source;

    // ---- yaml: number without integer representation ----
    #[test]
    fn test_yaml_large_float_stored_as_f64() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("float.yaml");
        std::fs::write(&path, "val: 1.23e10\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("val").unwrap());
        let v: f64 = config.get("val").unwrap();
        assert!(v > 1e9);
    }

    // ---- yaml: complex key (sequence key) ----
    #[test]
    fn test_yaml_sequence_key_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seq_key.yaml");
        std::fs::write(&path, "? [a, b]\n: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    // ---- yaml: null key ----
    #[test]
    fn test_yaml_null_key_becomes_null_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null_key.yaml");
        std::fs::write(&path, "~: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("null").unwrap());
    }

    // ---- yaml: bool key ----
    #[test]
    fn test_yaml_bool_key_becomes_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bool_key.yaml");
        std::fs::write(&path, "true: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("true").unwrap());
    }

    // ---- yaml: number key ----
    #[test]
    fn test_yaml_number_key_becomes_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("num_key.yaml");
        std::fs::write(&path, "42: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("42").unwrap());
    }

    // ---- yaml: yaml_scalar_to_string for null/bool/sequence/mapping ----
    #[test]
    fn test_yaml_mixed_sequence_with_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed_null.yaml");
        std::fs::write(&path, "vals:\n  - 1\n  - ~\n  - 3\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        // Mixed (int + null) → falls back to string
        assert!(config.contains("vals").unwrap());
    }

    #[test]
    fn test_yaml_homogeneous_scalar_sequences_keep_native_types() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("typed_sequences.yaml");
        std::fs::write(
            &path,
            r#"
ints:
  - 1
  - 2
floats:
  - 1.25
  - 2.5
flags:
  - true
  - false
"#,
        )
        .unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get_list::<i64>("ints").unwrap(), vec![1, 2]);
        assert_eq!(config.get_list::<f64>("floats").unwrap(), vec![1.25, 2.5]);
        assert_eq!(
            config.get_list::<bool>("flags").unwrap(),
            vec![true, false]
        );
    }

    #[test]
    fn test_yaml_nested_sequence_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_seq.yaml");
        std::fs::write(&path, "matrix:\n  - [1, 2]\n  - [3, 4]\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_yaml_sequence_with_mapping_returns_redacted_parse_error() {
        const SECRET_MARKER: &str = "RS_CONFIG_YAML_NESTED_SECRET_MARKER";
        let dir =
            tempfile::tempdir().expect("temporary directory should be created");
        let path = dir.path().join("seq_map.yaml");
        std::fs::write(
            &path,
            format!("items:\n  - password: {SECRET_MARKER}\n"),
        )
        .expect("nested YAML mapping fixture should be written");

        let source = YamlConfigSource::from_file(&path);
        let error = source.load().expect_err("nested YAML mapping should fail");

        let display = error.to_string();
        assert!(display.contains("items"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!format!("{error:?}").contains(SECRET_MARKER));
    }

    #[test]
    fn test_yaml_tagged_value_loads_inner_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tagged.yaml");
        std::fs::write(&path, "key: !custom hello\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();

        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("key").unwrap(), "hello");
    }

    #[test]
    fn test_yaml_scalar_values_respect_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scalar_final.yaml");
        std::fs::write(
            &path,
            r#"
locked_null: null
locked_bool: true
locked_int: 1
locked_float: 1.5
locked_string: value
locked_tagged: !custom tagged
"#,
        )
        .unwrap();

        for key in [
            "locked_null",
            "locked_bool",
            "locked_int",
            "locked_float",
            "locked_string",
            "locked_tagged",
        ] {
            let source = YamlConfigSource::from_file(&path);
            let mut config = Config::new();
            config.set(key, "old").unwrap();
            config.set_final(key, true).unwrap();

            let result = merge_source(&mut config, &source);

            assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
            assert_eq!(config.get::<String>(key).unwrap(), "old");
        }
    }

    #[test]
    fn test_yaml_sequences_respect_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sequence_final.yaml");
        std::fs::write(
            &path,
            r#"
locked_empty: []
locked_ints:
  - 1
  - 2
locked_floats:
  - 1.5
  - 2.5
locked_bools:
  - true
  - false
locked_strings:
  - one
  - two
locked_mixed:
  - 1
  - null
"#,
        )
        .unwrap();

        for key in [
            "locked_empty",
            "locked_ints",
            "locked_floats",
            "locked_bools",
            "locked_strings",
            "locked_mixed",
        ] {
            let source = YamlConfigSource::from_file(&path);
            let mut config = Config::new();
            config.set(key, vec!["old"]).unwrap();
            config.set_final(key, true).unwrap();

            let result = merge_source(&mut config, &source);

            assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
            assert_eq!(config.get::<Vec<String>>(key).unwrap(), vec!["old"]);
        }
    }
}
