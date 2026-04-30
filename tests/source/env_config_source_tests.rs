/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # `EnvConfigSource` tests

use qubit_config::{
    Config, ConfigError,
    source::{ConfigSource, EnvConfigSource},
};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes tests that mutate or read process environment variables.
fn env_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("environment test lock should not be poisoned")
}

// ============================================================================
// EnvConfigSource Tests
// ============================================================================

#[cfg(test)]
mod test_env_config_source {
    use super::*;

    #[test]
    fn test_load_all_env_vars() {
        let _guard = env_test_lock();
        // Set a unique test env var to verify it's loaded
        unsafe {
            std::env::set_var("QUBIT_TEST_UNIQUE_KEY_12345", "test_value");
        }

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(
            config.get_string("QUBIT_TEST_UNIQUE_KEY_12345").unwrap(),
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
        source.load(&mut config).unwrap();

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
        source.load(&mut config).unwrap();

        // After stripping prefix + lowercase + underscore→dot:
        // QTEST_HOST → host
        // QTEST_PORT → port
        assert_eq!(config.get_string("host").unwrap(), "myhost");
        assert_eq!(config.get_string("port").unwrap(), "9999");
        assert!(!config.contains("OTHER_VAR"));
        assert!(!config.contains("other.var"));

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
            std::env::set_var("MYAPP_SERVER_HOST", "app-host");
        }

        let source = EnvConfigSource::with_prefix("MYAPP_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // MYAPP_SERVER_HOST → server.host (strip prefix, lowercase, underscore→dot)
        assert_eq!(config.get_string("server.host").unwrap(), "app-host");
        assert!(!config.contains("MYAPP_SERVER_HOST"));

        unsafe {
            std::env::remove_var("MYAPP_SERVER_HOST");
        }
    }

    #[test]
    fn test_load_with_prefix_converts_underscores_to_dots() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("TAPP_DB_POOL_SIZE", "10");
        }

        let source = EnvConfigSource::with_prefix("TAPP_");
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("db.pool.size").unwrap(), "10");

        unsafe {
            std::env::remove_var("TAPP_DB_POOL_SIZE");
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
        source.load(&mut config).unwrap();

        assert_eq!(config.get_string("my.key").unwrap(), "val");

        unsafe {
            std::env::remove_var("LAPP_MY_KEY");
        }
    }

    #[test]
    fn test_default_creates_plain_source() {
        let _guard = env_test_lock();
        let source = EnvConfigSource::default();
        let mut config = Config::new();
        // Should not panic
        source.load(&mut config).unwrap();
    }

    #[test]
    fn test_with_options_no_strip_no_convert() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("RAWAPP_MY_KEY", "raw_val");
        }

        let source = EnvConfigSource::with_options("RAWAPP_", false, false, false);
        let mut config = Config::new();
        source.load(&mut config).unwrap();

        // Key kept as-is (prefix not stripped, no lowercase, no underscore conversion)
        assert_eq!(config.get_string("RAWAPP_MY_KEY").unwrap(), "raw_val");

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
        config.merge_from_source(&source).unwrap();

        assert_eq!(config.get_string("key").unwrap(), "merge_value");

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

        let key = "QUNICODE_BAD_VALUE";
        unsafe {
            std::env::set_var(key, OsString::from_vec(vec![b'o', 0xFF, b'k']));
        }

        let source = EnvConfigSource::with_prefix("QUNICODE_");
        let mut config = Config::new();
        let result = source.load(&mut config);

        unsafe {
            std::env::remove_var(key);
        }

        assert!(matches!(
            result,
            Err(ConfigError::ParseError(message))
                if message.contains(key) && message.contains("not valid Unicode")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_load_with_prefix_rejects_matching_non_unicode_env_key() {
        let _guard = env_test_lock();
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let key = OsString::from_vec(vec![
            b'Q', b'U', b'N', b'I', b'C', b'O', b'D', b'E', b'_', 0xFF,
        ]);
        unsafe {
            std::env::set_var(&key, "value");
        }

        let source = EnvConfigSource::with_prefix("QUNICODE_");
        let mut config = Config::new();
        let result = source.load(&mut config);

        unsafe {
            std::env::remove_var(&key);
        }

        assert!(matches!(
            result,
            Err(ConfigError::ParseError(message))
                if message.contains("Environment variable key")
                    && message.contains("not valid Unicode")
        ));
    }
}

#[cfg(test)]
mod test_env_coverage {
    use super::*;

    // ---- env: transform_key without strip_prefix ----
    #[test]
    fn test_env_config_source_with_options_no_strip() {
        let _guard = env_test_lock();
        use qubit_config::source::EnvConfigSource;
        unsafe {
            std::env::set_var("COVTEST_FOO", "bar");
        }
        let source = EnvConfigSource::with_options("COVTEST_", false, false, false);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        // Key kept as-is (not stripped, not lowercased, not converted)
        assert!(config.contains("COVTEST_FOO"));
        unsafe {
            std::env::remove_var("COVTEST_FOO");
        }
    }
}
