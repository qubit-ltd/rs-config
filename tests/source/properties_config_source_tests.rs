// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # `PropertiesConfigSource` tests

use std::path::PathBuf;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::ConfigSource;
use qubit_config::source::PropertiesConfigSource;

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
    config.merge_properties_from_source(source)
}

// ============================================================================
// PropertiesConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_properties_config_source {
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::PropertiesConfigSource;
    use super::fixture;
    use super::merge_source;

    // ---- parse_content unit tests ----

    #[test]
    fn test_parse_basic_key_value_equals() {
        let content = "key=value\nhost=localhost";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
        assert_eq!(pairs[1], ("host".to_string(), "localhost".to_string()));
    }

    #[test]
    fn test_parse_colon_separator() {
        let content = "key: value\nhost: localhost";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
        assert_eq!(pairs[1], ("host".to_string(), "localhost".to_string()));
    }

    #[test]
    fn test_parse_whitespace_separator() {
        let content = "server.port 8080\nhost\tlocalhost";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("server.port".to_string(), "8080".to_string()));
        assert_eq!(pairs[1], ("host".to_string(), "localhost".to_string()));
    }

    #[test]
    fn test_parse_whitespace_before_colon_separator() {
        let content = "key   :   value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_escaped_space_in_key_not_separator() {
        let content = "key\\ name=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key name".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_skips_hash_comments() {
        let content = "# This is a comment\nkey=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_skips_exclamation_comments() {
        let content = "! Another comment style\nkey=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_skips_blank_lines() {
        let content = "\n\nkey=value\n\n";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
    }

    #[test]
    fn test_parse_line_continuation() {
        let content = "key=val\\\nue";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_unicode_escape() {
        let content = "greeting=\\u4e2d\\u6587";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("greeting".to_string(), "中文".to_string()));
    }

    #[test]
    fn test_parse_unicode_surrogate_pair_escape() {
        let content = "emoji=\\uD83D\\uDE00";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("emoji".to_string(), "😀".to_string()));
    }

    #[test]
    fn test_parse_invalid_unicode_surrogates_are_preserved() {
        let content = "high=\\uD83D\\u0041\nlow=\\uDE00";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("high".to_string(), "\\uD83DA".to_string()));
        assert_eq!(pairs[1], ("low".to_string(), "\\uDE00".to_string()));
    }

    #[test]
    fn test_parse_empty_value() {
        let content = "empty=";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("empty".to_string(), "".to_string()));
    }

    #[test]
    fn test_parse_value_with_spaces() {
        let content = "key = value with spaces";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            pairs[0],
            ("key".to_string(), "value with spaces".to_string())
        );
    }

    #[test]
    fn test_parse_empty_content() {
        let pairs = PropertiesConfigSource::parse_content("").unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only_content() {
        let pairs = PropertiesConfigSource::parse_content("   \n\t\n").unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_only_comments() {
        let content = "# comment1\n# comment2\n! comment3";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_multiple_line_continuation() {
        let content = "key=first \\\n    second \\\n    third";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
        assert!(pairs[0].1.contains("first"));
        assert!(pairs[0].1.contains("second"));
        assert!(pairs[0].1.contains("third"));
    }

    #[test]
    fn test_parse_newline_escape() {
        let content = "key=line1\\nline2";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "line1\nline2".to_string()));
    }

    #[test]
    fn test_parse_tab_escape() {
        let content = "key=col1\\tcol2";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "col1\tcol2".to_string()));
    }

    #[test]
    fn test_parse_backslash_escape() {
        let content = "key=path\\\\to\\\\file";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key".to_string(), "path\\to\\file".to_string()));
    }

    #[test]
    fn test_parse_even_backslashes_before_separator() {
        let content = r"path\\=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("path\\".to_string(), "value".to_string()));
    }

    // ---- load from file tests ----

    #[test]
    fn test_load_basic_properties_file() {
        let source =
            PropertiesConfigSource::from_file(fixture("basic.properties"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
        assert_eq!(config.get::<String>("port").unwrap(), "8080");
        assert_eq!(config.get::<String>("debug").unwrap(), "true");
        assert_eq!(config.get::<String>("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<String>("app.version").unwrap(), "1.0.0");
    }

    #[test]
    fn test_load_multivalue_properties_file() {
        let source =
            PropertiesConfigSource::from_file(fixture("multivalue.properties"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("greeting").unwrap(), "中文测试");
        assert_eq!(config.get::<String>("empty.value").unwrap(), "");
        assert_eq!(config.get::<String>("colon.key").unwrap(), "colon value");
        assert_eq!(
            config.get::<String>("spaces.key").unwrap(),
            "value with spaces"
        );
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let source = PropertiesConfigSource::from_file(
            "/nonexistent/path/config.properties",
        );
        let result = source.load();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceIoError { .. })));
    }

    // ---- property-source merge integration ----

    #[test]
    fn test_merge_from_properties_config_source() {
        let source =
            PropertiesConfigSource::from_file(fixture("basic.properties"));
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert!(config.contains("host").unwrap());
        assert!(config.contains("port").unwrap());
    }
}

#[cfg(test)]
mod test_properties_edge_cases {

    use super::PropertiesConfigSource;

    #[test]
    fn test_properties_key_only_line() {
        let content = "standalone_key";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "standalone_key");
        assert_eq!(pairs[0].1, "");
    }

    // ---- properties: line continuation at EOF (no next line) ----
    #[test]
    fn test_properties_line_continuation_at_eof() {
        let content = "key=value\\";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "key");
        assert_eq!(pairs[0].1, "value");
    }

    #[test]
    fn test_properties_even_trailing_backslashes_do_not_continue() {
        let content = "path=C:\\\\\nnext=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("path".to_string(), "C:\\".to_string()));
        assert_eq!(pairs[1], ("next".to_string(), "value".to_string()));
    }

    #[test]
    fn test_properties_escaped_trailing_space_preserved_without_continuation() {
        let content = "key=value\\ \nnext=ok";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("key".to_string(), "value ".to_string()));
        assert_eq!(pairs[1], ("next".to_string(), "ok".to_string()));
    }

    #[test]
    fn test_properties_key_followed_by_trailing_whitespace_has_empty_value() {
        let content = "standalone_key   ";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("standalone_key".to_string(), "".to_string()));
    }

    // ---- properties: invalid unicode escape (partial) ----
    #[test]
    fn test_properties_invalid_unicode_escape_kept_as_is() {
        // \uXX is only 2 hex digits, not 4 → kept as-is
        let content = "key=\\uFF";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, "\\uFF");
    }

    // ---- properties: \r escape ----
    #[test]
    fn test_properties_carriage_return_escape() {
        let content = "key=line1\\rline2";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, "line1\rline2");
    }

    #[test]
    fn test_properties_form_feed_escape() {
        let content = "key=left\\fright";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, "left\u{000C}right");
    }

    // ---- properties: unknown escape sequence ----
    #[test]
    fn test_properties_unknown_escape_drops_backslash() {
        let content = "key=hello\\xworld";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].1, "helloxworld");
    }

    // ---- properties: escaped separator ----
    #[test]
    fn test_properties_escaped_equals_in_key() {
        let content = r"key\=name=value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("key=name".to_string(), "value".to_string()));
    }

    #[test]
    fn test_properties_escaped_separator_and_space_are_unescaped() {
        let content = "path\\:home:some\\ value\nhash\\#key=bang\\!value";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ("path:home".to_string(), "some value".to_string()),
        );
        assert_eq!(
            pairs[1],
            ("hash#key".to_string(), "bang!value".to_string()),
        );
    }
}
