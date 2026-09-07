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
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::EnvFileConfigSourceBuilder;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;

fn env_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment test lock should not be poisoned")
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
    use super::env_test_lock;
    use super::fixture;
    use super::merge_source;

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

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
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
        let _guard = env_test_lock();
        const KEY: &str = "RS_CONFIG_ENV_FILE_PROCESS_SECRET";
        unsafe {
            std::env::set_var(KEY, "process-secret");
        }

        let source = EnvFileConfigSource::from_content(format!("VALUE=${{{KEY}}}\n"));
        let config = source.load().expect(".env content should load");

        assert_eq!(config.get::<String>("VALUE").unwrap(), format!("${{{KEY}}}"));

        unsafe {
            std::env::remove_var(KEY);
        }
    }

    #[test]
    fn test_env_file_preserves_process_environment_placeholders_in_double_quotes() {
        let _guard = env_test_lock();
        const KEY: &str = "RS_CONFIG_ENV_FILE_DOUBLE_QUOTED_SECRET";
        unsafe {
            std::env::set_var(KEY, "process-secret");
        }

        let source = EnvFileConfigSource::from_content(format!("NAME=\"${KEY}\"\nBRACED=\"${{{KEY}}}\"\n"));
        let config = source.load().expect(".env content should load");

        assert_eq!(config.get::<String>("NAME").unwrap(), format!("${KEY}"));
        assert_eq!(config.get::<String>("BRACED").unwrap(), format!("${{{KEY}}}"));

        unsafe {
            std::env::remove_var(KEY);
        }
    }

    #[test]
    fn test_env_file_preserves_dotenv_escape_rules_around_substitutions() {
        let source = EnvFileConfigSource::from_content(
            "WEAK=\"line\\n${NAME}\"\nOUTSIDE=escaped\\ value\nSTRONG='${NAME}'\n",
        );

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
