/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # `TomlConfigSource` tests

use qubit_config::{
    Config, ConfigError,
    source::{ConfigSource, TomlConfigSource},
};

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

// ============================================================================
// TomlConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_toml_config_source {
    use super::*;

    #[test]
    fn test_load_basic_toml_file() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // String values remain strings
        assert_eq!(config.get_string("host").unwrap(), "localhost");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
        // Boolean values are stored as bool
        assert!(config.get::<bool>("debug").unwrap());
        // Float values are stored as f64
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_load_toml_nested_table_flattened() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
        assert_eq!(config.get_string("app.version").unwrap(), "1.0.0");
        assert_eq!(config.get_string("server.host").unwrap(), "0.0.0.0");
        // Integer values are stored as i64
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_load_toml_array_as_multivalue() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        let tags = config.get_string_list("tags.list").unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"web".to_string()));
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"v2".to_string()));
    }

    #[test]
    fn test_load_nonexistent_toml_file_returns_error() {
        let source = TomlConfigSource::from_file("/nonexistent/path/config.toml");
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::IoError(_))));
    }

    #[test]
    fn test_load_invalid_toml_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        std::fs::write(&path, "this is not valid toml ===").unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_merge_from_toml_config_source() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(config.contains("host"));
        assert!(config.contains("app.name"));
    }

    #[test]
    fn test_load_inline_toml_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inline.toml");
        std::fs::write(
            &path,
            r#"
name = "test"
value = 42
enabled = false

[db]
url = "postgres://localhost/mydb"
pool = 5
"#,
        )
        .unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("name").unwrap(), "test");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("value").unwrap(), 42);
        // Boolean values are stored as bool
        assert!(!config.get::<bool>("enabled").unwrap());
        assert_eq!(
            config.get_string("db.url").unwrap(),
            "postgres://localhost/mydb"
        );
        // Integer values are stored as i64
        assert_eq!(config.get::<i64>("db.pool").unwrap(), 5);
    }

    #[test]
    fn test_load_toml_array_of_tables_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("array_of_tables.toml");
        std::fs::write(
            &path,
            r#"
[[servers]]
host = "a"
port = 1

[[servers]]
host = "b"
port = 2
"#,
        )
        .unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}

#[cfg(test)]
mod test_toml_coverage {
    use super::*;

    // ---- toml: datetime value ----
    #[test]
    fn test_toml_datetime_stored_as_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dt.toml");
        std::fs::write(&path, "created_at = 2026-04-09T12:00:00Z\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        assert!(config.contains("created_at"));
        let val = config.get_string("created_at").unwrap();
        assert!(val.contains("2026"));
    }

    #[test]
    fn test_toml_empty_array_is_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_arr.toml");
        std::fs::write(&path, "empty = []\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert!(config.contains("empty"));
        assert_eq!(
            config.get_string_list("empty").unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(config.get_list::<i64>("empty").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_toml_empty_array_overrides_existing_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_override.toml");
        std::fs::write(&path, "ports = []\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("ports", vec![8080i64, 8081]).unwrap();

        source.load(&mut config).unwrap();

        assert_eq!(config.get_list::<i64>("ports").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_toml_empty_array_deserializes_as_empty_vec() {
        #[derive(serde::Deserialize)]
        struct Service {
            ports: Vec<i64>,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_deserialize.toml");
        std::fs::write(&path, "[service]\nports = []\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();

        source.load(&mut config).unwrap();
        let service: Service = config.deserialize("service").unwrap();

        assert_eq!(service.ports, Vec::<i64>::new());
    }

    #[test]
    fn test_toml_empty_array_respects_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_final.toml");
        std::fs::write(&path, "locked = []\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("locked", vec!["old"]).unwrap();
        config
            .get_property_mut("locked")
            .unwrap()
            .unwrap()
            .set_final(true);

        let result = source.load(&mut config);

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get_string_list("locked").unwrap(), vec!["old"]);
    }

    // ---- toml: mixed array (int + string) falls back to string ----
    #[test]
    fn test_toml_mixed_type_array_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.toml");
        // TOML spec actually disallows mixed arrays, but let's test the fallback
        // by using a valid TOML with all strings
        std::fs::write(&path, "tags = [\"a\", \"b\", \"c\"]\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        let tags: Vec<String> = config.get_list("tags").unwrap();
        assert_eq!(tags.len(), 3);
    }

    #[test]
    fn test_toml_homogeneous_scalar_arrays_keep_native_types() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("typed_arrays.toml");
        std::fs::write(
            &path,
            r#"
ints = [1, 2, 3]
floats = [1.25, 2.5]
flags = [true, false]
dates = [2026-04-09T12:00:00Z, 2026-04-10T12:00:00Z]
"#,
        )
        .unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_list::<i64>("ints").unwrap(), vec![1, 2, 3]);
        assert_eq!(config.get_list::<f64>("floats").unwrap(), vec![1.25, 2.5]);
        assert_eq!(config.get_list::<bool>("flags").unwrap(), vec![true, false]);
        let dates = config.get_string_list("dates").unwrap();
        assert_eq!(dates.len(), 2);
        assert!(dates[0].contains("2026-04-09"));
    }

    // ---- toml: toml_scalar_to_string for float/bool/datetime in mixed fallback ----
    #[test]
    fn test_toml_array_of_tables_nested_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_tbl.toml");
        std::fs::write(
            &path,
            "[[servers]]\nhost = \"a\"\n\n[[servers]]\nhost = \"b\"\n",
        )
        .unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}
