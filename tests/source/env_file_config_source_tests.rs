// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg(feature = "env-file")]

// # `EnvFileConfigSource` tests

use std::path::PathBuf;
use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::EnvFileConfigSourceBuilder;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;

#[test]
fn test_env_file_comment_does_not_change_substitution_state() {
    const CHILD_MODE: &str = "RS_CONFIG_ENV_FILE_COMMENT_TEST_CHILD";
    const SENTINEL: &str = "RS_CONFIG_ENV_FILE_COMMENT_TEST_SENTINEL";

    if std::env::var_os(CHILD_MODE).is_some() {
        let placeholder = format!("${{{SENTINEL}}}");
        let short_placeholder = format!("${SENTINEL}");
        let isolated_input = format!("A=#'\\''\nVALUE={placeholder}\n");
        let isolated_config = EnvFileConfigSource::from_content(isolated_input.clone())
            .load()
            .expect("comment and quote input should load");
        assert_eq!(isolated_config.get::<String>("VALUE").unwrap(), placeholder);
        assert_eq!(isolated_config.get::<String>("A").unwrap(), "");

        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("comment-quote-isolation.env");
        std::fs::write(&path, &isolated_input).expect("temporary dotenv file should be written");
        let isolated_file_config = EnvFileConfigSource::from_file(&path)
            .load()
            .expect("comment and quote file should load");
        assert_eq!(isolated_file_config.get::<String>("VALUE").unwrap(), placeholder);
        assert_eq!(isolated_file_config.get::<String>("A").unwrap(), "");

        let crlf_input = format!("A=#'\\''\r\nVALUE={placeholder}\r\n");
        std::fs::write(&path, &crlf_input).expect("CRLF dotenv file should be written");
        let crlf_file_config = EnvFileConfigSource::from_file(&path)
            .load()
            .expect("CRLF comment and quote file should load");
        assert_eq!(crlf_file_config.get::<String>("VALUE").unwrap(), placeholder);
        assert_eq!(crlf_file_config.get::<String>("A").unwrap(), "");

        let minimal_comment_separator_input = format!("A=\r#'\n='\nVALUE={placeholder}\n");
        let minimal_comment_separator_config =
            EnvFileConfigSource::from_content(minimal_comment_separator_input.clone())
                .load()
                .expect("minimal comment-separator input should load");
        assert_eq!(
            minimal_comment_separator_config.get::<String>("VALUE").unwrap(),
            placeholder
        );

        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("minimal-comment-separator.env");
        std::fs::write(&path, &minimal_comment_separator_input).expect("temporary dotenv file should be written");
        let minimal_comment_separator_file_config = EnvFileConfigSource::from_file(&path)
            .load()
            .expect("minimal comment-separator file should load");
        assert_eq!(
            minimal_comment_separator_file_config.get::<String>("VALUE").unwrap(),
            placeholder
        );
        let cases = [
            (format!("A=#'\\''\nVALUE={placeholder}\n"), placeholder.clone()),
            (
                format!("A=#'\\''\nVALUE={short_placeholder}\n"),
                short_placeholder.clone(),
            ),
            (format!("A=#'\\''\r\nVALUE={placeholder}\r\n"), placeholder.clone()),
            (
                format!("VALUE='first line\n{placeholder}'\n"),
                format!("first line\n{placeholder}"),
            ),
            ("VALUE='left\\right'\n".to_owned(), "left\\right".to_owned()),
            (
                format!("VALUE=prefix # ' \" dangling {placeholder}\n"),
                "prefix".to_owned(),
            ),
            (
                format!("# don't expand {placeholder}\nVALUE={placeholder}\n"),
                placeholder.clone(),
            ),
            (
                format!("  # \" don't expand {placeholder}\r\nVALUE={placeholder}\r\n"),
                placeholder.clone(),
            ),
            (
                format!("VALUE={placeholder} # don't \" expand {placeholder}\n"),
                placeholder.clone(),
            ),
            (
                format!("VALUE=hello\\ #{placeholder}\n"),
                format!("hello #{placeholder}"),
            ),
            (format!("VALUE=unicode\u{2003}#{placeholder}\n"), "unicode".to_owned()),
            (
                format!("{SENTINEL}=source-value\nVALUE=hello\\ #{placeholder}\n"),
                format!("hello #{placeholder}"),
            ),
            (format!("VALUE=hello #{placeholder}\n"), "hello".to_owned()),
            (format!("VALUE=abc#def{placeholder}\n"), format!("abc#def{placeholder}")),
            (format!("VALUE={short_placeholder}\n"), short_placeholder.clone()),
            (format!("VALUE=\"left#{placeholder}\"\n"), format!("left#{placeholder}")),
            (format!("VALUE=\"{short_placeholder}\"\n"), short_placeholder.clone()),
            (format!("VALUE='{placeholder}'\n"), placeholder.clone()),
            (format!("VALUE='{short_placeholder}'\n"), short_placeholder.clone()),
            (
                format!("VALUE=\"first line\n{placeholder}\"\n"),
                format!("first line\n{placeholder}"),
            ),
            (format!("VALUE=\\{placeholder}\n"), placeholder.clone()),
            (format!("export VALUE={placeholder}\n"), placeholder.clone()),
            (
                format!("{SENTINEL}=source-value\nVALUE={placeholder}\n"),
                placeholder.clone(),
            ),
        ];

        for (input, expected) in cases {
            let config = EnvFileConfigSource::from_content(input.clone())
                .load()
                .expect("dotenv sample should load");
            assert_eq!(
                config.get::<String>("VALUE").unwrap(),
                expected,
                "unexpected value for input {input:?}"
            );
        }

        let multiline_then_placeholder =
            format!("VALUE='first line\nsecond line'\nNEXT={placeholder}\nSHORT={short_placeholder}\n");
        let config = EnvFileConfigSource::from_content(multiline_then_placeholder)
            .load()
            .expect("assignment following a multiline value should load");
        assert_eq!(config.get::<String>("NEXT").unwrap(), placeholder);
        assert_eq!(config.get::<String>("SHORT").unwrap(), short_placeholder);

        let comment_with_unmatched_quote = format!("VALUE=prefix # ' \" dangling {placeholder}\nNEXT={placeholder}\n");
        let config = EnvFileConfigSource::from_content(comment_with_unmatched_quote)
            .load()
            .expect("assignment following a quoted comment should load");
        assert_eq!(config.get::<String>("NEXT").unwrap(), placeholder);

        let comment_with_paired_quotes =
            format!("VALUE=prefix # 'paired' \"paired\" {placeholder}\nNEXT={short_placeholder}\n");
        let config = EnvFileConfigSource::from_content(comment_with_paired_quotes)
            .load()
            .expect("assignment following a paired-quote comment should load");
        assert_eq!(config.get::<String>("NEXT").unwrap(), short_placeholder);

        let invalid_input = format!("VALUE=\"unterminated {placeholder}\n");
        let error = EnvFileConfigSource::from_content(invalid_input)
            .load()
            .expect_err("invalid quoted input must fail instead of expanding");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(
            !error
                .to_string()
                .contains(&std::env::var(SENTINEL).expect("sentinel should be set in child"))
        );
        return;
    }

    for environment_value in ["first-process-value", "second-process-value"] {
        let mut command = Command::new(std::env::current_exe().expect("test executable path should be available"));
        command
            .arg("--exact")
            .arg("test_env_file_comment_does_not_change_substitution_state")
            .env_clear()
            .env(CHILD_MODE, "1")
            .env(SENTINEL, environment_value);
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        let output = command.output().expect("child test process should start");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
        assert!(output.status.success(), "{stdout}\n{stderr}");
    }
}

fn run_with_environment(name: &str, key: &str) -> bool {
    const CHILD: &str = "RS_CONFIG_ENV_FILE_PROCESS_ENV_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(name)
        .arg("--nocapture")
        .env_clear()
        .env(CHILD, "1")
        .env(key, "process-secret");
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let output = command.output().expect("child test process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
    false
}

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
fn env_file_builder_default_loads_configured_content() {
    let builder_default: fn() -> EnvFileConfigSourceBuilder = Default::default;
    let source = std::hint::black_box(builder_default)()
        .content("COVERED=value\n")
        .build();

    assert_eq!(source.load().unwrap().get::<String>("COVERED").unwrap(), "value");
}

#[test]
fn test_env_file_parse_error_reports_parser_local_offset() {
    const SECRET: &str = "RS_CONFIG_DOTENV_OFFSET_SECRET";
    for prefix in ["", "VALID=1\n# preceding comment\n"] {
        let content = format!("{prefix}PASSWORD=\"{SECRET}\n");
        let source = EnvFileConfigSource::from_content(content);
        let expected_source = source.source_id();
        let error = source.load().expect_err("unterminated value must fail");
        assert!(matches!(&error, ConfigError::SourceParseError { .. }));
        assert_eq!(error.source_id(), Some(expected_source.as_str()));
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.contains("parser-local offset"));
        assert!(display.contains("not a file line or column"));
        assert!(!display.contains("line index"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET));
        assert!(!debug.contains(SECRET));
    }
}

// ============================================================================
// EnvFileConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_file_config_source {
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::EnvFileConfigSource;
    use super::PathBuf;
    use super::fixture;
    use super::merge_source;
    use super::run_with_environment;

    #[test]
    fn test_load_basic_env_file() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("HOST").unwrap(), "localhost");
        assert_eq!(config.get::<String>("PORT").unwrap(), "8080");
        assert_eq!(config.get::<String>("DEBUG").unwrap(), "true");
        assert_eq!(config.get::<String>("APP_NAME").unwrap(), "MyApp");
        assert_eq!(config.get::<String>("APP_VERSION").unwrap(), "1.0.0");
    }

    #[test]
    fn test_load_env_file_quoted_values() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("QUOTED_VALUE").unwrap(), "hello world");
        assert_eq!(config.get::<String>("SINGLE_QUOTED").unwrap(), "single quoted");
    }

    #[test]
    fn test_load_nonexistent_env_file_returns_error() {
        let source = EnvFileConfigSource::from_file("/nonexistent/path/.env");
        let result = source.load();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_load_inline_env_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(&path, "DB_HOST=db.example.com\nDB_PORT=5432\nDB_NAME=mydb\n").unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("DB_HOST").unwrap(), "db.example.com");
        assert_eq!(config.get::<String>("DB_PORT").unwrap(), "5432");
        assert_eq!(config.get::<String>("DB_NAME").unwrap(), "mydb");
    }

    #[test]
    fn test_load_env_file_respects_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("final.env");
        std::fs::write(&path, "LOCKED=new\n").unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("LOCKED", "old").unwrap();
        config.set_final("LOCKED", true).unwrap();

        let result = merge_source(&mut config, &source);

        assert!(
            matches!(result, Err(ConfigError::SourcePropertyIsFinal { source_id, .. }) if source_id == source.source_id())
        );
        assert_eq!(config.get::<String>("LOCKED").unwrap(), "old");
    }

    #[test]
    fn test_from_file_clone_keeps_debug_path() {
        let path = PathBuf::from("config.env");
        let source = EnvFileConfigSource::from_file(&path);
        let cloned = source.clone();

        assert_eq!(format!("{source:?}"), format!("{cloned:?}"));
        assert!(format!("{source:?}").contains("config.env"));
    }

    #[test]
    fn test_merge_from_env_file_config_source() {
        let source = EnvFileConfigSource::from_file(fixture("basic.env"));
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert!(config.contains("HOST").unwrap());
        assert!(config.contains("PORT").unwrap());
    }

    #[test]
    fn test_env_file_preserves_process_environment_placeholders() {
        const KEY: &str = "RS_CONFIG_ENV_FILE_PROCESS_SECRET";
        if !run_with_environment(
            "test_env_file_config_source::test_env_file_preserves_process_environment_placeholders",
            KEY,
        ) {
            return;
        }

        let source = EnvFileConfigSource::from_content(format!("VALUE=${{{KEY}}}\n"));
        let config = source.load().expect(".env content should load");

        assert_eq!(config.get::<String>("VALUE").unwrap(), format!("${{{KEY}}}"));
    }

    #[test]
    fn test_env_file_preserves_process_environment_placeholders_in_double_quotes() {
        const KEY: &str = "RS_CONFIG_ENV_FILE_DOUBLE_QUOTED_SECRET";
        if !run_with_environment(
            "test_env_file_config_source::test_env_file_preserves_process_environment_placeholders_in_double_quotes",
            KEY,
        ) {
            return;
        }

        let source = EnvFileConfigSource::from_content(format!("NAME=\"${KEY}\"\nBRACED=\"${{{KEY}}}\"\n"));
        let config = source.load().expect(".env content should load");

        assert_eq!(config.get::<String>("NAME").unwrap(), format!("${KEY}"));
        assert_eq!(config.get::<String>("BRACED").unwrap(), format!("${{{KEY}}}"));
    }

    #[test]
    fn test_env_file_preserves_dotenv_escape_rules_around_substitutions() {
        let source =
            EnvFileConfigSource::from_content("WEAK=\"line\\n${NAME}\"\nOUTSIDE=escaped\\ value\nSTRONG='${NAME}'\n");

        let config = source.load().expect("escaped dotenv content should load");

        assert_eq!(config.get::<String>("WEAK").unwrap(), "line\n${NAME}");
        assert_eq!(config.get::<String>("OUTSIDE").unwrap(), "escaped value");
        assert_eq!(config.get::<String>("STRONG").unwrap(), "${NAME}");
    }
}

#[cfg(test)]
mod test_env_file_edge_cases {

    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::EnvFileConfigSource;
    use super::SourceLimitKind;
    use super::SourceLimits;
    use super::merge_source;

    // ---- env_file: non-existent file returns IoError ----
    #[test]
    fn test_env_file_nonexistent_returns_io_error() {
        let source = EnvFileConfigSource::from_file("/nonexistent/path.env");
        let result = source.load();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_env_file_invalid_content_returns_redacted_parse_error() {
        const SECRET_MARKER: &str = "RS_CONFIG_DOTENV_SECRET_MARKER";
        let dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = dir.path().join("bad.env");
        std::fs::write(&path, format!("PASSWORD=\"{SECRET_MARKER}\n")).expect("invalid .env fixture should be written");

        let source = EnvFileConfigSource::from_file(&path);
        let error = source.load().expect_err("unterminated .env string should fail");

        assert!(matches!(&error, ConfigError::SourceParseError { .. }));
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.contains("bad.env"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!debug.contains(SECRET_MARKER));
    }

    #[test]
    fn test_env_file_invalid_key_includes_source_context() {
        let error = EnvFileConfigSource::from_content("invalid..key=value\n")
            .load()
            .expect_err("invalid .env key should fail");

        assert!(matches!(
            error,
            ConfigError::SourceParseError {
                source_id,
                path: Some(path),
                source_index: None,
                ..
            } if source_id == ".env:<memory>" && path == "invalid..key"
        ));
    }

    #[test]
    fn test_env_file_directory_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = EnvFileConfigSource::from_file(dir.path());

        source
            .load()
            .expect_err("loading a directory as an .env file should fail");
    }

    #[test]
    fn test_env_file_invalid_utf8_returns_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-utf8.env");
        std::fs::write(&path, [0xff]).unwrap();
        let source = EnvFileConfigSource::from_file(&path);

        assert!(matches!(source.load(), Err(ConfigError::SourceIoError { .. })));
    }

    #[test]
    fn test_env_file_empty_content_loads_empty_config() {
        let config = EnvFileConfigSource::from_content("")
            .load()
            .expect("empty dotenv content should load");

        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_env_file_input_limit_reports_observed_bytes() {
        let source = EnvFileConfigSource::builder()
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
    fn test_env_file_property_limit_counts_each_assignment() {
        let source = EnvFileConfigSource::builder()
            .content("FIRST=1\nSECOND=2\n")
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
    fn test_env_file_node_limit_counts_each_assignment() {
        let source = EnvFileConfigSource::builder()
            .content("FIRST=1\nSECOND=2\n")
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
    fn test_env_file_nesting_limit_rejects_deep_key() {
        let source = EnvFileConfigSource::builder()
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
    fn test_env_file_limit_failure_is_transactional() {
        let source = EnvFileConfigSource::builder()
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
