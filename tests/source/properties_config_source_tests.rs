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
use qubit_config::source::PropertiesConfigSourceBuilder;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn merge_source(config: &mut Config, source: &dyn ConfigSource) -> ConfigResult<()> {
    config.merge_properties_from_source(source)
}

#[test]
fn properties_builder_default_loads_configured_content() {
    let builder_default: fn() -> PropertiesConfigSourceBuilder = Default::default;
    let source = std::hint::black_box(builder_default)()
        .content("covered=value\n")
        .build();

    assert_eq!(source.load().unwrap().get::<String>("covered").unwrap(), "value");
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
    fn test_continuation_preserves_empty_physical_lines() {
        for first_end in ["\n", "\r", "\r\n"] {
            for blank_end in ["\n", "\r", "\r\n"] {
                // Adjacent CR and LF form one CRLF terminator.
                if first_end == "\r" && blank_end == "\n" {
                    continue;
                }
                let input = format!("a=one\\{first_end}{blank_end}b=two");
                let pairs = PropertiesConfigSource::parse_content(&input).unwrap();
                assert_eq!(
                    pairs,
                    vec![("a".to_owned(), "one".to_owned()), ("b".to_owned(), "two".to_owned())],
                    "input={input:?}"
                );
                let config = PropertiesConfigSource::from_content(input).load().unwrap();
                assert_eq!(config.get::<String>("a").unwrap(), "one");
                assert_eq!(config.get::<String>("b").unwrap(), "two");
            }
        }

        for input in ["a=one\\\r\n\r\nb=two", "a=one\\\r\n\nb=two"] {
            let pairs = PropertiesConfigSource::parse_content(input).unwrap();
            assert_eq!(
                pairs,
                vec![("a".to_owned(), "one".to_owned()), ("b".to_owned(), "two".to_owned())]
            );
        }
    }

    #[test]
    fn test_continuation_accumulates_escaped_backslashes() {
        let count = 512;
        let physical_line = format!("{}\n", "\\".repeat(3));
        let input = format!("key={}end", physical_line.repeat(count));
        let pairs = PropertiesConfigSource::parse_content(&input).unwrap();
        assert_eq!(pairs, vec![("key".to_owned(), format!("{}end", "\\".repeat(count)))]);
    }

    #[test]
    fn test_parse_unicode_escape() {
        let content = "greeting=\\u4e2d\\u6587";
        let pairs = PropertiesConfigSource::parse_content(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("greeting".to_string(), "中文".to_string()));
    }

    #[test]
    fn test_parse_malformed_unicode_escape_preserves_key_and_value() {
        for (content, expected) in [
            (r"key=\u+123", ("key", r"\u+123")),
            (r"\u+123=value", (r"\u+123", "value")),
            (r"key=\u+123\u0041", ("key", r"\u+123A")),
            ("key=\\u1é23", ("key", "\\u1é23")),
        ] {
            let pairs = PropertiesConfigSource::parse_content(content).unwrap();
            assert_eq!(
                pairs,
                vec![(expected.0.to_owned(), expected.1.to_owned())],
                "{content:?}"
            );
        }
    }

    #[test]
    fn test_parse_unicode_escape_accepts_ascii_hex_in_both_cases() {
        let pairs = PropertiesConfigSource::parse_content(r"key=\u00aF\u00Af").unwrap();
        assert_eq!(pairs, vec![("key".to_owned(), "\u{00af}\u{00af}".to_owned())]);
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
        assert_eq!(pairs[0], ("key".to_string(), "value with spaces".to_string()));
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
    fn properties_natural_lines_accept_cr_lf_and_crlf() {
        for input in ["a=1\rb=2", "a=1\nb=2", "a=1\r\nb=2", "a=1\rb=2\r\n"] {
            let pairs = PropertiesConfigSource::parse_content(input).unwrap();
            assert_eq!(
                pairs,
                vec![("a".to_owned(), "1".to_owned()), ("b".to_owned(), "2".to_owned())]
            );
        }
    }

    #[test]
    fn properties_keep_unicode_whitespace_in_keys_and_values() {
        let pairs = PropertiesConfigSource::parse_content("a\u{00a0}b=\u{00a0}value\nx=\u{3000}text").unwrap();
        assert_eq!(
            pairs,
            vec![
                ("a\u{00a0}b".to_owned(), "\u{00a0}value".to_owned()),
                ("x".to_owned(), "\u{3000}text".to_owned()),
            ]
        );
    }

    #[test]
    fn properties_format_whitespace_and_continuations_are_ascii_only() {
        let pairs = PropertiesConfigSource::parse_content(
            "  a\t=\u{000c}value\r\nb=left\\\rright\r\nc=left\\\r\n\u{00a0}right\nd=left\\rright",
        )
        .unwrap();

        assert_eq!(
            pairs,
            vec![
                ("a".to_owned(), "value".to_owned()),
                ("b".to_owned(), "leftright".to_owned()),
                ("c".to_owned(), "left\u{00a0}right".to_owned()),
                ("d".to_owned(), "left\rright".to_owned()),
            ]
        );
    }

    #[test]
    fn properties_source_load_preserves_unicode_whitespace() {
        let config = PropertiesConfigSource::from_content("a\u{00a0}b=\u{00a0}value\nx=\u{3000}text")
            .load()
            .unwrap();

        assert_eq!(config.get::<String>("a\u{00a0}b").unwrap(), "\u{00a0}value");
        assert_eq!(config.get::<String>("x").unwrap(), "\u{3000}text");
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
        let source = PropertiesConfigSource::from_file(fixture("basic.properties"));
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
        let source = PropertiesConfigSource::from_file(fixture("multivalue.properties"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("greeting").unwrap(), "中文测试");
        assert_eq!(config.get::<String>("empty.value").unwrap(), "");
        assert_eq!(config.get::<String>("colon.key").unwrap(), "colon value");
        assert_eq!(config.get::<String>("spaces.key").unwrap(), "value with spaces");
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let source = PropertiesConfigSource::from_file("/nonexistent/path/config.properties");
        let result = source.load();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceIoError { .. })));
    }

    // ---- property-source merge integration ----

    #[test]
    fn test_merge_from_properties_config_source() {
        let source = PropertiesConfigSource::from_file(fixture("basic.properties"));
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert!(config.contains("host").unwrap());
        assert!(config.contains("port").unwrap());
    }
}

#[cfg(test)]
mod test_properties_edge_cases {

    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::PropertiesConfigSource;
    use super::SourceLimitKind;
    use super::SourceLimits;
    use super::merge_source;

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
        assert_eq!(pairs[0], ("path:home".to_string(), "some value".to_string()),);
        assert_eq!(pairs[1], ("hash#key".to_string(), "bang!value".to_string()),);
    }

    #[test]
    fn test_properties_empty_content_loads_empty_config() {
        let config = PropertiesConfigSource::from_content("")
            .load()
            .expect("empty properties content should load");

        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_properties_invalid_key_returns_source_parse_error() {
        let error = PropertiesConfigSource::from_content("invalid..key=value\n")
            .load()
            .expect_err("invalid property key should fail");

        assert!(matches!(
            error,
            ConfigError::SourceParseError {
                source_id,
                path: Some(path),
                source_index: None,
                ..
            } if source_id == "properties:<memory>" && path == "invalid..key"
        ));
    }

    #[test]
    fn test_properties_input_limit_reports_observed_bytes() {
        let source = PropertiesConfigSource::builder()
            .content("KEY=VALUE")
            .limits(SourceLimits::builder().max_input_bytes(8).build())
            .build();

        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::InputBytes,
                limit: 8,
                observed_at_least: 9,
                ..
            })
        ));
    }

    #[test]
    fn test_properties_property_limit_counts_duplicate_assignments() {
        let source = PropertiesConfigSource::builder()
            .content("same=1\nsame=2\n")
            .limits(SourceLimits::builder().max_properties(1).build())
            .build();

        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::PropertyCount,
                limit: 1,
                observed_at_least: 2,
                ..
            })
        ));
    }

    #[test]
    fn test_properties_node_limit_counts_each_assignment() {
        let source = PropertiesConfigSource::builder()
            .content("first=1\nsecond=2\n")
            .limits(SourceLimits::builder().max_nodes(1).build())
            .build();

        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::NodeCount,
                limit: 1,
                observed_at_least: 2,
                ..
            })
        ));
    }

    #[test]
    fn test_properties_nesting_limit_rejects_deep_key() {
        let source = PropertiesConfigSource::builder()
            .content("server.port=8080\n")
            .limits(SourceLimits::builder().max_nesting_depth(1).build())
            .build();

        assert!(matches!(
            source.load(),
            Err(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::NestingDepth,
                limit: 1,
                observed_at_least: 2,
                ..
            })
        ));
    }

    #[test]
    fn test_properties_directory_path_returns_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = PropertiesConfigSource::from_file(dir.path());

        assert!(matches!(source.load(), Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_properties_invalid_utf8_returns_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-utf8.properties");
        std::fs::write(&path, [0xff]).unwrap();
        let source = PropertiesConfigSource::from_file(&path);

        assert!(matches!(source.load(), Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_properties_limit_failure_is_transactional() {
        let source = PropertiesConfigSource::builder()
            .content("first=1\nsecond=2\n")
            .limits(SourceLimits::builder().max_properties(1).build())
            .build();
        let mut config = Config::new();
        config.set("existing", "kept").unwrap();

        let result = merge_source(&mut config, &source);

        assert!(matches!(
            result,
            Err(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::PropertyCount,
                ..
            })
        ));
        assert_eq!(config.len(), 1);
        assert_eq!(config.get::<String>("existing").unwrap(), "kept");
        assert!(!config.contains("first").unwrap());
    }
}
