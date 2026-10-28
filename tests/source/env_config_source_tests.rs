// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # `EnvConfigSource` tests

use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::ConfigSource;
use qubit_config::source::EnvConfigOptions;
use qubit_config::source::EnvConfigOptionsBuilder;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::EnvConfigSourceBuilder;
use qubit_config::source::SourceLimitKind;
use qubit_config::source::SourceLimits;
use qubit_redact::RedactionCompletion;
use qubit_redact::Redactor;

fn run_with_env(name: &str, values: &[(&str, &str)]) -> bool {
    const CHILD: &str = "QUBIT_ENV_CONFIG_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(name)
        .arg("--nocapture")
        .env_clear()
        .env(CHILD, "1");
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    for (key, value) in values {
        command.env(key, value);
    }
    let output = command.output().expect("child test process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
    false
}

#[cfg(unix)]
fn run_with_os_env(name: &str, values: &[(std::ffi::OsString, std::ffi::OsString)]) -> bool {
    const CHILD: &str = "QUBIT_ENV_CONFIG_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(name)
        .arg("--nocapture")
        .env_clear()
        .env(CHILD, "1");
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    for (key, value) in values {
        command.env(key, value);
    }
    let output = command.output().expect("child test process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
    false
}

fn merge_source(config: &mut Config, source: &dyn ConfigSource) -> ConfigResult<()> {
    config.merge_properties_from_source(source)
}

#[test]
fn environment_builders_expose_default_and_limit_configuration() {
    let options_default: fn() -> EnvConfigOptionsBuilder = Default::default;
    let source_default: fn() -> EnvConfigSourceBuilder = Default::default;
    let limits: fn(EnvConfigSourceBuilder, SourceLimits) -> EnvConfigSourceBuilder = EnvConfigSourceBuilder::limits;

    let options = std::hint::black_box(options_default)().build();
    let source = std::hint::black_box(limits)(
        std::hint::black_box(source_default)(),
        SourceLimits::builder().max_properties(0).build(),
    )
    .options(options)
    .build();
    assert_eq!(source.limits().max_properties(), 0);
}

#[cfg(unix)]
#[test]
fn normalized_conflicts_are_stable_through_public_source_loading() {
    const CHILD_MODE: &str = "QUBIT_ENV_CONFLICT_CHILD";
    const PREFIX: &str = "QUBIT_ENV_CONFLICT_REVIEW_";
    const NAMES: [&str; 3] = [
        "QUBIT_ENV_CONFLICT_REVIEW_FOO__BAR",
        "QUBIT_ENV_CONFLICT_REVIEW_foo__bar",
        "QUBIT_ENV_CONFLICT_REVIEW_foo.bar",
    ];

    if std::env::var_os(CHILD_MODE).is_some() {
        let source = EnvConfigSource::from_prefix(PREFIX);
        let mut target = Config::new();
        target.set("existing", "old").unwrap();
        let before = target.clone();
        let error = merge_source(&mut target, &source).expect_err("the three normalized names must conflict");
        assert_eq!(target, before);
        assert!(matches!(
            &error,
            ConfigError::KeyConflict {
                source_id: Some(source_id),
                path,
                existing,
                incoming,
            } if source_id == "process environment"
                && path == "foo.bar"
                && existing == "environment variable 'QUBIT_ENV_CONFLICT_REVIEW_FOO__BAR'"
                && incoming == "environment variable 'QUBIT_ENV_CONFLICT_REVIEW_foo.bar'"
        ));
        let rendered = format!("{error:?} {error}");
        for secret in ["private-marker-a", "private-marker-b", "private-marker-c"] {
            assert!(!rendered.contains(secret));
        }
        return;
    }

    let mut command = Command::new(std::env::current_exe().expect("test executable should be available"));
    command
        .arg("--exact")
        .arg("normalized_conflicts_are_stable_through_public_source_loading")
        .env_clear()
        .env(CHILD_MODE, "1")
        .env(NAMES[0], "private-marker-a")
        .env(NAMES[1], "private-marker-b")
        .env(NAMES[2], "private-marker-c");
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let output = command.output().expect("child test process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("running 1 test"), "{stdout}\n{stderr}");
    assert!(output.status.success(), "{stdout}\n{stderr}");
}

#[test]
fn env_source_limits_are_charged_once_and_merge_is_transactional() {
    const PREFIX: &str = "QUBIT_ENV_LIMIT_REVIEW_";
    const FIRST: &str = "QUBIT_ENV_LIMIT_REVIEW_FIRST";
    const SECOND: &str = "QUBIT_ENV_LIMIT_REVIEW_SECOND";
    if !run_with_env(
        "env_source_limits_are_charged_once_and_merge_is_transactional",
        &[(FIRST, "one"), (SECOND, "two")],
    ) {
        return;
    }

    let total_bytes = FIRST.len() + "one".len() + SECOND.len() + "two".len();
    let exact_limits = SourceLimits::builder()
        .max_input_bytes(total_bytes)
        .max_properties(2)
        .max_nodes(2)
        .build();
    let exact_source = EnvConfigSource::builder()
        .options(EnvConfigOptions::builder().prefix(PREFIX).build())
        .limits(exact_limits)
        .build();
    assert!(
        exact_source.load().is_ok(),
        "exactly sufficient budgets should load both entries"
    );

    let too_small_limits = [
        (
            SourceLimits::builder().max_input_bytes(total_bytes - 1).build(),
            SourceLimitKind::InputBytes,
        ),
        (
            SourceLimits::builder().max_properties(1).build(),
            SourceLimitKind::PropertyCount,
        ),
        (SourceLimits::builder().max_nodes(1).build(), SourceLimitKind::NodeCount),
    ];
    for (limits, expected_kind) in too_small_limits {
        let source = EnvConfigSource::builder()
            .options(EnvConfigOptions::builder().prefix(PREFIX).build())
            .limits(limits)
            .build();
        let mut target = Config::new();
        target.set("existing", "old").unwrap();
        let before = target.clone();
        assert!(matches!(
            merge_source(&mut target, &source),
            Err(ConfigError::SourceLimitExceeded { kind, .. }) if kind == expected_kind
        ));
        assert_eq!(target, before);
    }
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
    use super::RedactionCompletion;
    use super::Redactor;
    use super::merge_source;
    use super::run_with_env;
    #[cfg(unix)]
    use super::run_with_os_env;

    /// Verifies the default environment policy redacts an ordinary UTF-8
    /// sensitive pair.
    #[test]
    fn test_default_env_policy_redacts_sensitive_utf8_value() {
        let redacted = Redactor::standard().redact_env("APP_PASSWORD", "plain-secret");
        let rendered = redacted.text().as_str();

        assert_eq!(redacted.summary().completion(), RedactionCompletion::Complete);
        assert_eq!(rendered, "APP_PASSWORD=<redacted>");
        assert!(!rendered.contains("plain-secret"));
    }

    #[test]
    fn test_load_all_env_vars() {
        if !run_with_env(
            "test_env_config_source::test_load_all_env_vars",
            &[("QUBIT_TEST_UNIQUE_KEY_12345", "test_value")],
        ) {
            return;
        }

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(
            config.get::<String>("QUBIT_TEST_UNIQUE_KEY_12345").unwrap(),
            "test_value"
        );
    }

    #[test]
    fn test_load_env_var_bool_zero_can_be_read_as_bool() {
        if !run_with_env(
            "test_env_config_source::test_load_env_var_bool_zero_can_be_read_as_bool",
            &[("IS_USE_PREFIX", "0")],
        ) {
            return;
        }

        let source = EnvConfigSource::new();
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert!(!config.get::<bool>("IS_USE_PREFIX").unwrap());
    }

    #[test]
    fn test_load_with_prefix_filters_vars() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_filters_vars",
            &[
                ("QTEST_HOST", "myhost"),
                ("QTEST_PORT", "9999"),
                ("OTHER_VAR", "should_not_appear"),
            ],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QTEST_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // After stripping prefix + lowercase + double-underscore→dot:
        // QTEST_HOST → host
        // QTEST_PORT → port
        assert_eq!(config.get::<String>("host").unwrap(), "myhost");
        assert_eq!(config.get::<String>("port").unwrap(), "9999");
        assert!(!config.contains("OTHER_VAR").unwrap());
        assert!(!config.contains("other.var").unwrap());
    }

    #[test]
    fn test_load_with_prefix_strips_prefix() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_strips_prefix",
            &[("MYAPP_SERVER__HOST", "app-host")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("MYAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // MYAPP_SERVER__HOST → server.host (strip prefix, lowercase,
        // double-underscore→dot)
        assert_eq!(config.get::<String>("server.host").unwrap(), "app-host");
        assert!(!config.contains("MYAPP_SERVER__HOST").unwrap());
    }

    #[test]
    fn test_load_with_prefix_converts_double_underscores_to_dots() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_converts_double_underscores_to_dots",
            &[("TAPP_DB__POOL_SIZE", "10")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("TAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("db.pool_size").unwrap(), "10");
    }

    #[test]
    fn test_load_with_prefix_lowercases_keys() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_lowercases_keys",
            &[("LAPP_MY_KEY", "val")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("LAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("my_key").unwrap(), "val");
    }

    #[test]
    fn test_load_with_prefix_preserves_single_underscore() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_preserves_single_underscore",
            &[("UAPP_A___B", "value")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("UAPP_");
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        assert_eq!(config.get::<String>("a._b").unwrap(), "value");
    }

    #[test]
    fn test_default_creates_plain_source() {
        if !run_with_env(
            "test_env_config_source::test_default_creates_plain_source",
            &[("QUBIT_ENV_CONFIG_DEFAULT_TEST_KEY", "default_value")],
        ) {
            return;
        }

        let source = EnvConfigSource::default();
        let config = source.load().unwrap();
        assert_eq!(
            config.get::<String>("QUBIT_ENV_CONFIG_DEFAULT_TEST_KEY").unwrap(),
            "default_value"
        );
    }

    #[test]
    fn test_with_options_no_strip_no_convert() {
        if !run_with_env(
            "test_env_config_source::test_with_options_no_strip_no_convert",
            &[("RAWAPP_MY_KEY", "raw_val")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_options(EnvConfigOptions::builder().prefix("RAWAPP_").build());
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();

        // Key kept as-is (prefix not stripped, no lowercase, no underscore
        // conversion)
        assert_eq!(config.get::<String>("RAWAPP_MY_KEY").unwrap(), "raw_val");
    }

    #[test]
    fn test_merge_from_env_config_source() {
        if !run_with_env(
            "test_env_config_source::test_merge_from_env_config_source",
            &[("MERGETEST_KEY", "merge_value")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("MERGETEST_");
        let mut config = Config::new();
        config.merge_properties_from_source(&source).unwrap();

        assert_eq!(config.get::<String>("key").unwrap(), "merge_value");
    }

    #[cfg(unix)]
    #[test]
    fn test_load_with_prefix_rejects_non_unicode_env_value() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        const SECRET_MARKER: &str = "RS_CONFIG_ENV_VALUE_SECRET_MARKER";
        const INJECTION_MARKER: &str = "FORGED_ENV_VALUE_LINE";
        let key = "QUNICODE_BAD_VALUE";
        let mut raw_value = SECRET_MARKER.as_bytes().to_vec();
        raw_value.extend_from_slice(format!("\n{INJECTION_MARKER}\r").as_bytes());
        raw_value.push(0xFF);
        if !run_with_os_env(
            "test_env_config_source::test_load_with_prefix_rejects_non_unicode_env_value",
            &[(OsString::from(key), OsString::from_vec(raw_value))],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QUNICODE_");
        let error = source.load().expect_err("non-Unicode environment value should fail");

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
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        const SECRET_MARKER: &str = "RS_CONFIG_ENV_KEY_SECRET_MARKER";
        const INJECTION_MARKER: &str = "FORGED_ENV_KEY_LINE";
        let mut raw_key = b"QUNICODE_".to_vec();
        raw_key.extend_from_slice(SECRET_MARKER.as_bytes());
        raw_key.extend_from_slice(format!("\n{INJECTION_MARKER}\r").as_bytes());
        raw_key.push(0xFF);
        let key = OsString::from_vec(raw_key);
        if !run_with_os_env(
            "test_env_config_source::test_load_with_prefix_rejects_matching_non_unicode_env_key",
            &[(key.clone(), OsString::from("value"))],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QUNICODE_");
        let error = source.load().expect_err("non-Unicode environment key should fail");

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
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_rejects_empty_normalized_key",
            &[("QEMPTY_", "value")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QEMPTY_");
        let mut config = Config::new();
        config.set("existing", "old").unwrap();
        let result = merge_source(&mut config, &source);

        assert!(matches!(
            result,
            Err(ConfigError::KeyConflict { path, .. }) if path.is_empty()
        ));
        assert_eq!(config.get::<String>("existing").unwrap(), "old");
        assert_eq!(config.len(), 1);
    }

    #[test]
    fn test_load_with_prefix_rejects_malformed_normalized_dotted_key() {
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_rejects_malformed_normalized_dotted_key",
            &[
                ("QMAL___LEADING", "leading"),
                ("QMAL_DB____HOST", "middle"),
                ("QMAL_DB__", "trailing"),
            ],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QMAL_");
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);

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
        const KEY: &str = "QUBIT_INVALID_RAW..KEY";
        if !run_with_env(
            "test_env_config_source::test_load_invalid_raw_key_includes_source_context",
            &[(KEY, "value")],
        ) {
            return;
        }

        let error = EnvConfigSource::new()
            .load()
            .expect_err("invalid raw environment key should fail");

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
        if !run_with_env(
            "test_env_config_source::test_load_with_prefix_rejects_duplicate_normalized_key",
            &[("QDUP_key", "two"), ("QDUP_KEY", "one")],
        ) {
            return;
        }

        let source = EnvConfigSource::from_prefix("QDUP_");
        let mut config = Config::new();
        let result = merge_source(&mut config, &source);

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
    use super::merge_source;
    use super::run_with_env;

    // ---- env: transform_key without strip_prefix ----
    #[test]
    fn test_env_config_source_with_options_no_strip() {
        use qubit_config::source::EnvConfigSource;
        if !run_with_env(
            "test_env_edge_cases::test_env_config_source_with_options_no_strip",
            &[("COVTEST_FOO", "bar")],
        ) {
            return;
        }
        let source = EnvConfigSource::from_options(EnvConfigOptions::builder().prefix("COVTEST_").build());
        let mut config = Config::new();
        merge_source(&mut config, &source).unwrap();
        // Key kept as-is (not stripped, not lowercased, not converted)
        assert!(config.contains("COVTEST_FOO").unwrap());
    }

    #[test]
    fn test_env_config_options_apply_each_key_transform_independently() {
        if !run_with_env(
            "test_env_edge_cases::test_env_config_options_apply_each_key_transform_independently",
            &[("OPTTEST_Mixed_Key", "value")],
        ) {
            return;
        }

        let mut config = Config::new();
        let source = EnvConfigSource::from_options(EnvConfigOptions::builder().prefix("OPTTEST_").build());
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("OPTTEST_Mixed_Key").unwrap());

        let mut config = Config::new();
        let source =
            EnvConfigSource::from_options(EnvConfigOptions::builder().prefix("OPTTEST_").strip_prefix().build());
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("Mixed_Key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::from_options(
            EnvConfigOptions::builder()
                .prefix("OPTTEST_")
                .double_underscores_to_dots()
                .build(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("OPTTEST_Mixed_Key").unwrap());

        let mut config = Config::new();
        let source =
            EnvConfigSource::from_options(EnvConfigOptions::builder().prefix("OPTTEST_").lowercase_keys().build());
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("opttest_mixed_key").unwrap());

        let mut config = Config::new();
        let source = EnvConfigSource::from_options(
            EnvConfigOptions::builder()
                .prefix("OPTTEST_")
                .strip_prefix()
                .double_underscores_to_dots()
                .lowercase_keys()
                .build(),
        );
        merge_source(&mut config, &source).unwrap();
        assert!(config.contains("mixed_key").unwrap());
    }
}
