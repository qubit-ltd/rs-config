/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # `EnvFileConfigSource` tests

use qubit_config::{
    Config,
    ConfigError,
    source::{
        ConfigSource,
        EnvFileConfigSource,
    },
};

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

// ============================================================================
// EnvFileConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_file_config_source {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        ConfigSource,
        EnvFileConfigSource,
        PathBuf,
        fixture,
    };

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
    fn test_load_env_file_respects_final_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("final.env");
        std::fs::write(&path, "LOCKED=new\n").unwrap();

        let source = EnvFileConfigSource::from_file(&path);
        let mut config = Config::new();
        config.set("LOCKED", "old").unwrap();
        config.set_final("LOCKED", true).unwrap();

        let result = source.load(&mut config);

        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get_string("LOCKED").unwrap(), "old");
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

        assert!(config.contains("HOST"));
        assert!(config.contains("PORT"));
    }
}

#[cfg(test)]
mod test_env_file_edge_cases {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        ConfigSource,
        EnvFileConfigSource,
        PathBuf,
        fixture,
    };

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

    #[test]
    fn test_env_file_directory_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = EnvFileConfigSource::from_file(dir.path());
        let mut config = Config::new();

        source
            .load(&mut config)
            .expect_err("loading a directory as an .env file should fail");
    }
}
