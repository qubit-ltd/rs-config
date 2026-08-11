// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # `EnvConfigSource` tests

use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvConfigOptions;
use qubit_config::source::EnvConfigSource;
use qubit_redact::EnvRedactor;

/// Serializes tests that mutate or read process environment variables.
fn env_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment test lock should not be poisoned")
}

fn merge_source(config: &mut Config, source: &dyn ConfigSource) -> ConfigResult<()> {
    config.merge_properties_from_source(source)
}

// ============================================================================
// EnvConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_config_source {
    use super::Config;
    use super::ConfigError;
    use super::ConfigSource;
    use super::EnvConfigOptions;
    use super::EnvConfigSource;
    use super::EnvRedactor;
    use super::env_test_lock;
    use super::merge_source;

    /// Verifies the default environment policy redacts an ordinary UTF-8
    /// sensitive pair.
    #[test]
    fn test_default_env_policy_redacts_sensitive_utf8_value() {
        let rendered = EnvRedactor::default()
            .redact_os_pair("APP_PASSWORD".as_ref(), "plain-secret".as_ref())
            .to_string();

        assert_eq!(rendered, "APP_PASSWORD=<redacted>");
        assert!(!rendered.contains("plain-secret"));
    }

    #[test]
    fn test_load_all_env_vars() {
        let _guard = env_test_lock();
        // Set a unique test env var to verify it's loaded
        unsafe {
            std::env::set_var("QUBIT_TEST_UNIQUE_KEY_12345", "test_value");
        }

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(
            config.get::<String>("QUBIT_TEST_UNIQUE_KEY_12345").unwrap(),
            "test_value"
        );

        unsafe {
            std::env::remove_var("QUBIT_TEST_UNIQUE_KEY_12345");
        }
    }

    #[test]
    fn test_load_env_var_bool_zero_can_be_read_as_bool() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("IS_USE_PREFIX", "0");
        }

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert!(!config.get::<bool>("IS_USE_PREFIX").unwrap());

        unsafe {
            std::env::remove_var("IS_USE_PREFIX");
        }
    }

    #[test]
    fn test_load_with_prefix_filters_vars() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QTEST_HOST", "myhost");
            std::env::set_var("QTEST_PORT", "9999");
            std::env::set_var("OTHER_VAR", "should_not_appear");
        }

        let source = EnvConfigSource::with_prefix("QTEST_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // After stripping prefix + lowercase + double-underscore→dot:
        // QTEST_HOST → host
        // QTEST_PORT → port
        assert_eq!(config.get::<String>("host").unwrap(), "myhost");
        assert_eq!(config.get::<String>("port").unwrap(), "9999");
        assert!(!config.contains("OTHER_VAR").unwrap());
        assert!(!config.contains("other.var").unwrap());

        unsafe {
            std::env::remove_var("QTEST_HOST");
            std::env::remove_var("QTEST_PORT");
            std::env::remove_var("OTHER_VAR");
        }
    }

    #[test]
    fn test_load_with_prefix_strips_prefix() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("MYAPP_SERVER__HOST", "app-host");
        }

        let source = EnvConfigSource::with_prefix("MYAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // MYAPP_SERVER__HOST → server.host (strip prefix, lowercase,
        // double-underscore→dot)
        assert_eq!(config.get::<String>("server.host").unwrap(), "app-host");
        assert!(!config.contains("MYAPP_SERVER__HOST").unwrap());

        unsafe {
            std::env::remove_var("MYAPP_SERVER__HOST");
        }
    }

    #[test]
    fn test_load_with_prefix_converts_double_underscores_to_dots() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("TAPP_DB__POOL_SIZE", "10");
        }

        let source = EnvConfigSource::with_prefix("TAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("db.pool_size").unwrap(), "10");

        unsafe {
            std::env::remove_var("TAPP_DB__POOL_SIZE");
        }
    }

    #[test]
    fn test_load_with_prefix_lowercases_keys() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("LAPP_MY_KEY", "val");
        }

        let source = EnvConfigSource::with_prefix("LAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("my_key").unwrap(), "val");

        unsafe {
            std::env::remove_var("LAPP_MY_KEY");
        }
    }

    #[test]
    fn test_load_with_prefix_preserves_single_underscore() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("UAPP_A___B", "value");
        }

        let source = EnvConfigSource::with_prefix("UAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("a._b").unwrap(), "value");

        unsafe {
            std::env::remove_var("UAPP_A___B");
        }
    }

    #[test]
    fn test_default_creates_plain_source() {
        let _guard = env_test_lock();
        let source = EnvConfigSource::default();
        let mut config = Config::new();
        // Should not panic
        merge_source(&mut config, &source).unwrap();
    }

    #[test]
    fn test_with_options_no_strip_no_convert() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("RAWAPP_MY_KEY", "raw_val");
        }

        let source = EnvConfigSource::with_options(EnvConfigOptions::new().prefix("RAWAPP_"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // Key kept as-is (prefix not stripped, no lowercase, no underscore
        // conversion)
        assert_eq!(config.get::<String>("RAWAPP_MY_KEY").unwrap(), "raw_val");

        unsafe {
            std::env::remove_var("RAWAPP_MY_KEY");
        }
    }

    #[test]
    fn test_merge_from_env_config_source() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("MERGETEST_KEY", "merge_value");
        }

        let source = EnvConfigSource::with_prefix("MERGETEST_");
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert_eq!(config.get::<String>("key").unwrap(), "merge_value");

        unsafe {
            std::env::remove_var("MERGETEST_KEY");
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_load_with_prefix_rejects_non_unicode_env_value() {
        let _guard = env_test_lock();
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        const SECRET_MARKER: &str = "RS_CONFIG_ENV_VALUE_SECRET_MARKER";
        const INJECTION_MARKER: &str = "FORGED_ENV_VALUE_LINE";
        let key = "QUNICODE_BAD_VALUE";
        let mut raw_value = SECRET_MARKER.as_bytes().to_vec();
        raw_value.extend_from_slice(format!("\n{INJECTION_MARKER}\r").as_bytes());
        raw_value.push(0xFF);
        unsafe {
            std::env::set_var(key, OsString::from_vec(raw_value));
        }

        let source = EnvConfigSource::with_prefix("QUNICODE_");
        let error = source
            .load()
            .expect_err("non-Unicode environment value should fail");

        unsafe {
            std::env::remove_var(key);
        }

        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.contains(key));
        assert!(display.contains("not valid Unicode"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!display.contains(INJECTION_MARKER));
        assert!(!display.contains('\n'));
        assert!(!display.contains('\r'));
        assert!(!debug.contains(SECRET_MARKER));
        assert!(!debug.contains(INJECTION_MARKER));
        assert!(!debug.contains('\n'));
        assert!(!debug.contains('\r'));
    }

    #[cfg(unix)]
    #[test]
    fn test_load_with_prefix_rejects_matching_non_unicode_env_key() {
        let _guard = env_test_lock();
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        const SECRET_MARKER: &str = "RS_CONFIG_ENV_KEY_SECRET_MARKER";
        const INJECTION_MARKER: &str = "FORGED_ENV_KEY_LINE";
        let mut raw_key = b"QUNICODE_".to_vec();
        raw_key.extend_from_slice(SECRET_MARKER.as_bytes());
        raw_key.extend_from_slice(format!("\n{INJECTION_MARKER}\r").as_bytes());
        raw_key.push(0xFF);
        let key = OsString::from_vec(raw_key);
        unsafe {
            std::env::set_var(&key, "value");
        }

        let source = EnvConfigSource::with_prefix("QUNICODE_");
        let error = source
            .load()
            .expect_err("non-Unicode environment key should fail");

        unsafe {
            std::env::remove_var(&key);
        }

        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.contains("Environment variable key"));
        assert!(display.contains("not valid Unicode"));
        assert!(display.contains("<redacted>"));
        assert!(!display.contains(SECRET_MARKER));
        assert!(!display.contains(INJECTION_MARKER));
        assert!(!display.contains('\n'));
        assert!(!display.contains('\r'));
        assert!(!debug.contains(SECRET_MARKER));
        assert!(!debug.contains(INJECTION_MARKER));
        assert!(!debug.contains('\n'));
        assert!(!debug.contains('\r'));
    }

    #[test]
    fn test_load_with_prefix_rejects_empty_normalized_key() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QEMPTY_", "value");
        }

        let source = EnvConfigSource::with_prefix("QEMPTY_");
        let mut config = Config::new();
        config.set("existing", "old").unwrap();
        let result = merge_source(&mut config, &source);

        unsafe {
            std::env::remove_var("QEMPTY_");
        }

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path.is_empty()
        ));
        assert_eq!(config.get::<String>("existing").unwrap(), "old");
        assert_eq!(config.len(), 1);
    }

    #[test]
    fn test_load_with_prefix_rejects_malformed_normalized_dotted_key() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QMAL___LEADING", "leading");
            std::env::set_var("QMAL_DB____HOST", "middle");
            std::env::set_var("QMAL_DB__", "trailing");
        }

        let source = EnvConfigSource::with_prefix("QMAL_");
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);

        unsafe {
            std::env::remove_var("QMAL___LEADING");
            std::env::remove_var("QMAL_DB____HOST");
            std::env::remove_var("QMAL_DB__");
        }

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. })
                if path == ".leading" || path == "db..host" || path == "db."
        ));
        assert!(config.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_load_invalid_raw_key_includes_source_context() {
        let _guard = env_test_lock();
        const KEY: &str = "QUBIT_INVALID_RAW..KEY";

        unsafe {
            std::env::set_var(KEY, "value");
        }

        let error = EnvConfigSource::new()
            .load()
            .expect_err("invalid raw environment key should fail");

        unsafe {
            std::env::remove_var(KEY);
        }

        assert!(matches!(
            error,
            ConfigError::SourceParseError {
                source_id,
                path: Some(path),
                source_index: None,
                ..
            } if source_id == "process environment" && path == KEY
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_load_with_prefix_rejects_duplicate_normalized_key() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QDUP_key", "two");
            std::env::set_var("QDUP_KEY", "one");
        }

        let source = EnvConfigSource::with_prefix("QDUP_");
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);

        unsafe {
            std::env::remove_var("QDUP_KEY");
            std::env::remove_var("QDUP_key");
        }

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict {
                source_id: Some(source_id),
                path,
                existing,
                incoming,
            }) if source_id == "process environment"
                && path == "key"
                && existing == "environment variable 'QDUP_KEY'"
                && incoming == "environment variable 'QDUP_key'"
        ));
        assert!(config.is_empty());
    }
}

#[cfg(test)]
mod test_env_edge_cases {
    use super::Config;
    use super::EnvConfigOptions;
    use super::EnvConfigSource;
    use super::env_test_lock;
    use super::merge_source;

    // ---- env: transform_key without strip_prefix ----
    #[test]
    fn test_env_config_source_with_options_no_strip() {
        let _guard = env_test_lock();
        use qubit_config::source::EnvConfigSource;
        unsafe {
            std::env::set_var("COVTEST_FOO", "bar");
        }
        let source = EnvConfigSource::with_options(EnvConfigOptions::new().prefix("COVTEST_"));
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        // Key kept as-is (not stripped, not lowercased, not converted)
        assert!(config.contains("COVTEST_FOO").unwrap());
        unsafe {
            std::env::remove_var("COVTEST_FOO");
        }
    }

    #[test]
    fn test_env_config_options_apply_each_key_transform_independently() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("OPTTEST_Mixed_Key", "value");
        }

        let mut config = Config::new();
        let source = EnvConfigSource::with_options(EnvConfigOptions::new().prefix("OPTTEST_"));
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("OPTTEST_Mixed_Key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::with_options(
            EnvConfigOptions::new().prefix("OPTTEST_").strip_prefix(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("Mixed_Key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::with_options(
            EnvConfigOptions::new()
                .prefix("OPTTEST_")
                .double_underscores_to_dots(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("OPTTEST_Mixed_Key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::with_options(
            EnvConfigOptions::new().prefix("OPTTEST_").lowercase_keys(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("opttest_mixed_key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::with_options(
            EnvConfigOptions::new()
                .prefix("OPTTEST_")
                .strip_prefix()
                .double_underscores_to_dots()
                .lowercase_keys(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("mixed_key").unwrap());

        unsafe {
            std::env::remove_var("OPTTEST_Mixed_Key");
        }
    }
}
