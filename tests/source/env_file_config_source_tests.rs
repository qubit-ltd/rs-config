// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg(feature = "env-file")]

// # `EnvFileConfigSource` tests

use qubit_config::{
    Config, ConfigError, ConfigResult,
    source::{ConfigSource, EnvFileConfigSource},
};

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

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
    config.merge_from_source(source)
}

// ============================================================================
// EnvFileConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_file_config_source {
    #[allow(unused_imports)]
    use super::{
        Config, ConfigError, ConfigSource, EnvFileConfigSource, Mutex, MutexGuard, OnceLock,
        PathBuf, env_test_lock, fixture, merge_source,
    };

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
        assert_eq!(
            config.get::<String>("SINGLE_QUOTED").unwrap(),
            "single quoted"
        );
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
        std::fs::write(
            &path,
            "DB_HOST=db.example.com\nDB_PORT=5432\nDB_NAME=mydb\n",
        )
        .unwrap();

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
        config.merge_from_source(&source).unwrap();

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

        assert_eq!(
            config.get::<String>("VALUE").unwrap(),
            format!("${{{KEY}}}")
        );

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

        let source =
            EnvFileConfigSource::from_content(format!("NAME=\"${KEY}\"\nBRACED=\"${{{KEY}}}\"\n"));
        let config = source.load().expect(".env content should load");

        assert_eq!(config.get::<String>("NAME").unwrap(), format!("${KEY}"));
        assert_eq!(
            config.get::<String>("BRACED").unwrap(),
            format!("${{{KEY}}}")
        );

        unsafe {
            std::env::remove_var(KEY);
        }
    }
}

#[cfg(test)]
mod test_env_file_edge_cases {
    #[allow(unused_imports)]
    use super::{
        Config, ConfigError, ConfigSource, EnvFileConfigSource, PathBuf, fixture, merge_source,
    };

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
        std::fs::write(&path, format!("PASSWORD=\"{SECRET_MARKER}\n"))
            .expect("invalid .env fixture should be written");

        let source = EnvFileConfigSource::from_file(&path);
        let error = source
            .load()
            .expect_err("unterminated .env string should fail");

        assert!(matches!(&error, ConfigError::SourceParseError { .. }));
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.contains("bad.env"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!debug.contains(SECRET_MARKER));
    }

    #[test]
    fn test_env_file_directory_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = EnvFileConfigSource::from_file(dir.path());

        source
            .load()
            .expect_err("loading a directory as an .env file should fail");
    }
}
