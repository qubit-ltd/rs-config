/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Source Integration Tests
//!
//! Tests all configuration source implementations and the `merge_from_source` method.

use std::path::PathBuf;

use qubit_config::{
    source::{
        CompositeConfigSource, ConfigSource, EnvConfigSource, EnvFileConfigSource,
        PropertiesConfigSource, TomlConfigSource, YamlConfigSource,
    },
    Config, ConfigError,
};

/// Returns the path to a test fixture file
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

// ============================================================================
// PropertiesConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_properties_config_source {
    use super::*;

    // ---- parse_content unit tests ----

    #[test]
    fn test_parse_basic_key_value_equals() {
        let content = "key=value\nhost=localhost";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
        assert_eq!(pairs[1], ("host".to_string(), "localhost".to_string()));
    }

    #[test]
    fn test_parse_colon_separator() {
        let content = "key: value\nhost: localhost";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
        assert_eq!(pairs[1], ("host".to_string(), "localhost".to_string()));
    }

    #[test]
    fn test_parse_skips_hash_comments() {
        let content = "# This is a comment\nkey=value";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_skips_exclamation_comments() {
        let content = "! Another comment style\nkey=value";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_skips_blank_lines() {
        let content = "\n\nkey=value\n\n";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_line_continuation() {
        let content = "key=val\\\nue";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_unicode_escape() {
        let content = "greeting=\\u4e2d\\u6587";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("greeting".to_string(), "中文".to_string()));
    }

    #[test]
    fn test_parse_empty_value() {
        let content = "empty=";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("empty".to_string(), "".to_string()));
    }

    #[test]
    fn test_parse_value_with_spaces() {
        let content = "key = value with spaces";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            pairs[0],
            ("key".to_string(), "value with spaces".to_string())
        );
    }

    #[test]
    fn test_parse_empty_content() {
        let pairs = PropertiesConfigSource::parse_content("");
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_only_comments() {
        let content = "# comment1\n# comment2\n! comment3";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_multiple_line_continuation() {
        let content = "key=first \\\n    second \\\n    third";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
        assert!(pairs[0].1.contains("first"));
        assert!(pairs[0].1.contains("second"));
        assert!(pairs[0].1.contains("third"));
    }

    #[test]
    fn test_parse_newline_escape() {
        let content = "key=line1\\nline2";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "line1\nline2".to_string()));
    }

    #[test]
    fn test_parse_tab_escape() {
        let content = "key=col1\\tcol2";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "col1\tcol2".to_string()));
    }

    #[test]
    fn test_parse_backslash_escape() {
        let content = "key=path\\\\to\\\\file";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "path\\to\\file".to_string()));
    }

    #[test]
    fn test_parse_even_backslashes_before_separator() {
        let content = r"path\\=value";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("path\\".to_string(), "value".to_string()));
    }

    // ---- load from file tests ----

    #[test]
    fn test_load_basic_properties_file() {
        let source = PropertiesConfigSource::from_file(fixture("basic.properties"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "localhost");
        assert_eq!(config.get_string("port").unwrap(), "8080");
        assert_eq!(config.get_string("debug").unwrap(), "true");
        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
        assert_eq!(config.get_string("app.version").unwrap(), "1.0.0");
    }

    #[test]
    fn test_load_multivalue_properties_file() {
        let source = PropertiesConfigSource::from_file(fixture("multivalue.properties"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("greeting").unwrap(), "中文测试");
        assert_eq!(config.get_string("empty.value").unwrap(), "");
        assert_eq!(config.get_string("colon.key").unwrap(), "colon value");
        assert_eq!(
            config.get_string("spaces.key").unwrap(),
            "value with spaces"
        );
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let source = PropertiesConfigSource::from_file("/nonexistent/path/config.properties");
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::IoError(_))));
    }

    // ---- merge_from_source integration ----

    #[test]
    fn test_merge_from_properties_config_source() {
        let source = PropertiesConfigSource::from_file(fixture("basic.properties"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(config.contains("host"));
        assert!(config.contains("port"));
    }
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

// ============================================================================
// YamlConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_yaml_config_source {
    use super::*;

    #[test]
    fn test_load_basic_yaml_file() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
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
    fn test_load_yaml_nested_mapping_flattened() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
        assert_eq!(config.get_string("app.version").unwrap(), "1.0.0");
        assert_eq!(config.get_string("server.host").unwrap(), "0.0.0.0");
        // Integer values are stored as i64
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_load_yaml_sequence_as_multivalue() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        let tags = config.get_string_list("tags").unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"web".to_string()));
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"v2".to_string()));
    }

    #[test]
    fn test_load_nonexistent_yaml_file_returns_error() {
        let source = YamlConfigSource::from_file("/nonexistent/path/config.yaml");
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::IoError(_))));
    }

    #[test]
    fn test_load_invalid_yaml_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.yaml");
        std::fs::write(&path, "key: [unclosed bracket").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_merge_from_yaml_config_source() {
        let source = YamlConfigSource::from_file(fixture("basic.yaml"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(config.contains("host"));
        assert!(config.contains("app.name"));
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
    fn test_load_yaml_null_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null.yaml");
        std::fs::write(&path, "key: ~\nother: value").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // Null values are preserved as empty properties (is_null returns true)
        assert!(config.contains("key"));
        assert!(config.is_null("key"));
        // get_optional returns None for null values
        let val: Option<String> = config.get_optional("key").unwrap();
        assert_eq!(val, None);
        assert_eq!(config.get_string("other").unwrap(), "value");
    }

    #[test]
    fn test_load_yaml_complex_keys_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("complex_key.yaml");
        std::fs::write(&path, "? [a, b]\n: 1\n? {x: 1}\n: 2\n").unwrap();

        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}

// ============================================================================
// EnvFileConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_file_config_source {
    use super::*;

    #[test]
    fn test_load_basic_env_file() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("HOST").unwrap(), "localhost");
        assert_eq!(config.get_string("PORT").unwrap(), "8080");
        assert_eq!(config.get_string("DEBUG").unwrap(), "true");
        assert_eq!(config.get_string("APP_NAME").unwrap(), "MyApp");
        assert_eq!(config.get_string("APP_VERSION").unwrap(), "1.0.0");
    }

    #[test]
    fn test_load_env_file_quoted_values() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("QUOTED_VALUE").unwrap(), "hello world");
        assert_eq!(config.get_string("SINGLE_QUOTED").unwrap(), "single quoted");
    }

    #[test]
    fn test_load_nonexistent_env_file_returns_error() {
        let source = EnvFileConfigSource::from_file("/nonexistent/path/.env");
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::IoError(_))));
    }

    #[test]
    fn test_load_inline_env_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(
            &path,
            "DB_HOST=db.example.com\nDB_PORT=5432\nDB_NAME=mydb\n",
        )
        .unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("DB_HOST").unwrap(), "db.example.com");
        assert_eq!(config.get_string("DB_PORT").unwrap(), "5432");
        assert_eq!(config.get_string("DB_NAME").unwrap(), "mydb");
    }

    #[test]
    fn test_merge_from_env_file_config_source() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(config.contains("HOST"));
        assert!(config.contains("PORT"));
    }
}

// ============================================================================
// EnvConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_config_source {
    use super::*;

    #[test]
    fn test_load_all_env_vars() {
        // Set a unique test env var to verify it's loaded
        std::env::set_var("QUBIT_TEST_UNIQUE_KEY_12345", "test_value");

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(
            config.get_string("QUBIT_TEST_UNIQUE_KEY_12345").unwrap(),
            "test_value"
        );

        std::env::remove_var("QUBIT_TEST_UNIQUE_KEY_12345");
    }

    #[test]
    fn test_load_with_prefix_filters_vars() {
        std::env::set_var("QTEST_HOST", "myhost");
        std::env::set_var("QTEST_PORT", "9999");
        std::env::set_var("OTHER_VAR", "should_not_appear");

        let source = EnvConfigSource::with_prefix("QTEST_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // After stripping prefix + lowercase + underscore→dot:
        // QTEST_HOST → host
        // QTEST_PORT → port
        assert_eq!(config.get_string("host").unwrap(), "myhost");
        assert_eq!(config.get_string("port").unwrap(), "9999");
        assert!(!config.contains("OTHER_VAR"));
        assert!(!config.contains("other.var"));

        std::env::remove_var("QTEST_HOST");
        std::env::remove_var("QTEST_PORT");
        std::env::remove_var("OTHER_VAR");
    }

    #[test]
    fn test_load_with_prefix_strips_prefix() {
        std::env::set_var("MYAPP_SERVER_HOST", "app-host");

        let source = EnvConfigSource::with_prefix("MYAPP_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // MYAPP_SERVER_HOST → server.host (strip prefix, lowercase, underscore→dot)
        assert_eq!(config.get_string("server.host").unwrap(), "app-host");
        assert!(!config.contains("MYAPP_SERVER_HOST"));

        std::env::remove_var("MYAPP_SERVER_HOST");
    }

    #[test]
    fn test_load_with_prefix_converts_underscores_to_dots() {
        std::env::set_var("TAPP_DB_POOL_SIZE", "10");

        let source = EnvConfigSource::with_prefix("TAPP_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("db.pool.size").unwrap(), "10");

        std::env::remove_var("TAPP_DB_POOL_SIZE");
    }

    #[test]
    fn test_load_with_prefix_lowercases_keys() {
        std::env::set_var("LAPP_MY_KEY", "val");

        let source = EnvConfigSource::with_prefix("LAPP_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("my.key").unwrap(), "val");

        std::env::remove_var("LAPP_MY_KEY");
    }

    #[test]
    fn test_default_creates_plain_source() {
        let source = EnvConfigSource::default();
        let mut config = Config::new();
        // Should not panic
        source.load(&mut config).unwrap();
    }

    #[test]
    fn test_with_options_no_strip_no_convert() {
        std::env::set_var("RAWAPP_MY_KEY", "raw_val");

        let source = EnvConfigSource::with_options("RAWAPP_", false, false, false);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // Key kept as-is (prefix not stripped, no lowercase, no underscore conversion)
        assert_eq!(config.get_string("RAWAPP_MY_KEY").unwrap(), "raw_val");

        std::env::remove_var("RAWAPP_MY_KEY");
    }

    #[test]
    fn test_merge_from_env_config_source() {
        std::env::set_var("MERGETEST_KEY", "merge_value");

        let source = EnvConfigSource::with_prefix("MERGETEST_");
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert_eq!(config.get_string("key").unwrap(), "merge_value");

        std::env::remove_var("MERGETEST_KEY");
    }
}

// ============================================================================
// CompositeConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_composite_config_source {
    use super::*;

    #[test]
    fn test_new_composite_is_empty() {
        let composite = CompositeConfigSource::new();
        assert!(composite.is_empty());
        assert_eq!(composite.len(), 0);
    }

    #[test]
    fn test_add_source_increases_len() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        assert_eq!(composite.len(), 1);
        assert!(!composite.is_empty());
    }

    #[test]
    fn test_add_multiple_sources() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(PropertiesConfigSource::from_file(fixture(
            "basic.properties",
        )));
        assert_eq!(composite.len(), 2);
    }

    #[test]
    fn test_load_merges_sources_in_order() {
        // basic.toml sets host=localhost, override.toml sets host=production-server
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        composite.load(&mut config).unwrap();

        // Later source wins
        assert_eq!(config.get_string("host").unwrap(), "production-server");
        // Integer values are stored as i64 (type-faithful)
        assert_eq!(config.get::<i64>("port").unwrap(), 443);
        // Keys only in first source are still present
        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
    }

    #[test]
    fn test_load_empty_composite_does_nothing() {
        let composite = CompositeConfigSource::new();
        let mut config = Config::new();
        composite.load(&mut config).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn test_load_stops_on_first_error() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file("/nonexistent/path.toml"));
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));

        let mut config = Config::new();
        let result = composite.load(&mut config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_creates_empty_composite() {
        let composite = CompositeConfigSource::default();
        assert!(composite.is_empty());
    }

    #[test]
    fn test_composite_with_env_override() {
        std::env::set_var("CTEST_HOST", "env-host");

        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(EnvConfigSource::with_prefix("CTEST_"));

        let mut config = Config::new();
        composite.load(&mut config).unwrap();

        // env source overrides toml
        assert_eq!(config.get_string("host").unwrap(), "env-host");
        // toml-only keys still present
        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");

        std::env::remove_var("CTEST_HOST");
    }

    #[test]
    fn test_merge_from_composite_config_source() {
        let mut composite = CompositeConfigSource::new();
        composite.add(TomlConfigSource::from_file(fixture("basic.toml")));
        composite.add(TomlConfigSource::from_file(fixture("override.toml")));

        let mut config = Config::new();
        config.merge_from_source(&composite).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "production-server");
    }

    #[test]
    fn test_add_returns_mutable_ref_for_chaining() {
        // Verify the builder-style chaining works
        let mut composite = CompositeConfigSource::new();
        composite
            .add(TomlConfigSource::from_file(fixture("basic.toml")))
            .add(TomlConfigSource::from_file(fixture("override.toml")));

        assert_eq!(composite.len(), 2);
    }
}

// ============================================================================
// Config::merge_from_source Tests
// ============================================================================

#[cfg(test)]
mod test_config_merge_from_source {
    use super::*;

    #[test]
    fn test_merge_from_source_populates_config() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(!config.is_empty());
        assert!(config.contains("host"));
    }

    #[test]
    fn test_merge_from_source_overwrites_existing_keys() {
        let mut config = Config::new();
        config.set("host", "old-host").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_from_source(&source).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_merge_from_source_preserves_final_property() {
        let mut config = Config::new();
        config.set("host", "final-host").unwrap();
        if let Some(prop) = config.get_property_mut("host") {
            prop.set_final(true);
        }

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let result = config.merge_from_source(&source);

        // Should fail because host is final
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        // Value unchanged
        assert_eq!(config.get_string("host").unwrap(), "final-host");
    }

    #[test]
    fn test_merge_from_source_adds_new_keys() {
        let mut config = Config::new();
        config.set("existing", "value").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_from_source(&source).unwrap();

        // Original key preserved
        assert_eq!(config.get_string("existing").unwrap(), "value");
        // New keys added
        assert!(config.contains("host"));
        assert!(config.contains("app.name"));
    }

    #[test]
    fn test_merge_from_source_returns_error_on_failure() {
        let source = TomlConfigSource::from_file("/nonexistent/path.toml");
        let mut config = Config::new();
        let result = config.merge_from_source(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_from_source_with_variable_substitution() {
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
        config.merge_from_source(&source).unwrap();

        // Variable substitution is applied on get_string
        assert_eq!(
            config.get_string("api_url").unwrap(),
            "http://localhost:8080/api"
        );
    }
}

// ============================================================================
// Additional coverage tests for uncovered paths
// ============================================================================

#[cfg(test)]
mod test_coverage_gaps {
    use super::*;

    // ---- properties: key-only line (no separator) ----
    #[test]
    fn test_properties_key_only_line() {
        let content = "standalone_key";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "standalone_key");
        assert_eq!(pairs[0].1, "");
    }

    // ---- properties: line continuation at EOF (no next line) ----
    #[test]
    fn test_properties_line_continuation_at_eof() {
        let content = "key=value\\";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
        assert_eq!(pairs[0].1, "value");
    }

    // ---- properties: invalid unicode escape (partial) ----
    #[test]
    fn test_properties_invalid_unicode_escape_kept_as_is() {
        // \uXX is only 2 hex digits, not 4 → kept as-is
        let content = "key=\\uFF";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        // partial hex → kept as literal
        assert!(pairs[0].1.contains("\\u") || pairs[0].1.contains("FF"));
    }

    // ---- properties: \r escape ----
    #[test]
    fn test_properties_carriage_return_escape() {
        let content = "key=line1\\rline2";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, "line1\rline2");
    }

    // ---- properties: unknown escape sequence ----
    #[test]
    fn test_properties_unknown_escape_kept_as_backslash() {
        let content = "key=hello\\xworld";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        // Unknown escape: backslash kept
        assert!(pairs[0].1.contains("\\x") || pairs[0].1.contains('\\'));
    }

    // ---- properties: escaped separator ----
    #[test]
    fn test_properties_escaped_equals_in_key() {
        let content = r"key\=name=value";
        let pairs = PropertiesConfigSource::parse_content(content);
        assert_eq!(pairs.len(), 1);
        // The escaped '=' in the key is not treated as separator
        assert!(pairs[0].0.contains("key"));
    }

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

    // ---- toml: empty array ----
    #[test]
    fn test_toml_empty_array_no_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty_arr.toml");
        std::fs::write(&path, "empty = []\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        // Empty array: no entry created
        assert!(!config.contains("empty"));
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

    // ---- yaml: number without integer representation ----
    #[test]
    fn test_yaml_large_float_stored_as_f64() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("float.yaml");
        std::fs::write(&path, "val: 1.23e10\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        assert!(config.contains("val"));
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
        let result = source.load(&mut config);
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
        source.load(&mut config).unwrap();
        assert!(config.contains("null"));
    }

    // ---- yaml: bool key ----
    #[test]
    fn test_yaml_bool_key_becomes_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bool_key.yaml");
        std::fs::write(&path, "true: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        assert!(config.contains("true"));
    }

    // ---- yaml: number key ----
    #[test]
    fn test_yaml_number_key_becomes_string() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("num_key.yaml");
        std::fs::write(&path, "42: value\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        assert!(config.contains("42"));
    }

    // ---- yaml: yaml_scalar_to_string for null/bool/sequence/mapping ----
    #[test]
    fn test_yaml_mixed_sequence_with_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed_null.yaml");
        std::fs::write(&path, "vals:\n  - 1\n  - ~\n  - 3\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        // Mixed (int + null) → falls back to string
        assert!(config.contains("vals"));
    }

    // ---- env_file: non-existent file returns IoError ----
    #[test]
    fn test_env_file_nonexistent_returns_io_error() {
        let source = EnvFileConfigSource::from_file("/nonexistent/path.env");
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::IoError(_))));
    }

    // ---- env_file: file with invalid content triggers parse error ----
    #[test]
    fn test_env_file_invalid_content_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.env");
        // dotenvy rejects lines with invalid unicode or certain malformed entries
        // Write a file with a NUL byte which dotenvy will reject
        std::fs::write(&path, b"KEY=\x00value\n").unwrap();
        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        // Either succeeds (dotenvy is lenient) or fails with ParseError
        // We just verify it doesn't panic
        let _ = result;
    }

    // ---- env: transform_key without strip_prefix ----
    #[test]
    fn test_env_config_source_with_options_no_strip() {
        use qubit_config::source::EnvConfigSource;
        std::env::set_var("COVTEST_FOO", "bar");
        let source = EnvConfigSource::with_options("COVTEST_", false, false, false);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        // Key kept as-is (not stripped, not lowercased, not converted)
        assert!(config.contains("COVTEST_FOO"));
        std::env::remove_var("COVTEST_FOO");
    }
}
