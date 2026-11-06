// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::ffi::OsString;

use super::EnvConfigSource;
use crate::ConfigError;
use crate::source::SourceLimits;
use crate::source::SourceLoadContext;

fn source_context(limits: SourceLimits) -> SourceLoadContext<'static> {
    SourceLoadContext::new("process environment".to_string(), limits)
}

fn entries(names: &[&str]) -> Vec<(OsString, OsString)> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| (OsString::from(*name), OsString::from(format!("SECRET_VALUE_{index}"))))
        .collect()
}

#[test]
fn test_three_colliding_names_choose_same_pair_for_every_permutation() {
    let names = ["REVIEW_FOO__BAR", "REVIEW_foo__bar", "REVIEW_foo.bar"];
    let permutations = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

    for permutation in permutations {
        let ordered_names = permutation.map(|index| names[index]);
        let source = EnvConfigSource::from_prefix("REVIEW_");
        let mut context = source_context(SourceLimits::default());

        let error = source
            .load_environment_entries(entries(&ordered_names), &mut context)
            .unwrap_err();
        assert!(matches!(
            error,
            ConfigError::KeyConflict {
                source_id: Some(source_id),
                path,
                existing,
                incoming,
            } if source_id == "process environment"
                && path == "foo.bar"
                && existing == "environment variable 'REVIEW_FOO__BAR'"
                && incoming == "environment variable 'REVIEW_foo.bar'"
        ));
    }
}

#[test]
fn test_conflict_selection_uses_smallest_normalized_key_and_redacts_values() {
    let names = ["REVIEW_ZED__KEY", "REVIEW_zed.key", "REVIEW_FOO__BAR", "REVIEW_foo.bar"];
    let source = EnvConfigSource::from_prefix("REVIEW_");

    for first in 0..names.len() {
        for second in 0..names.len() {
            for third in 0..names.len() {
                for fourth in 0..names.len() {
                    if first == second
                        || first == third
                        || first == fourth
                        || second == third
                        || second == fourth
                        || third == fourth
                    {
                        continue;
                    }
                    let ordered_names = [names[first], names[second], names[third], names[fourth]];
                    let mut context = source_context(SourceLimits::default());
                    let error = source
                        .load_environment_entries(entries(&ordered_names), &mut context)
                        .unwrap_err();
                    assert!(matches!(
                        &error,
                        ConfigError::KeyConflict {
                            path,
                            existing,
                            incoming,
                            ..
                        } if path == "foo.bar"
                            && existing == "environment variable 'REVIEW_FOO__BAR'"
                            && incoming == "environment variable 'REVIEW_foo.bar'"
                    ));
                    let rendered = format!("{error:?} {error}");
                    for marker in ["SECRET_VALUE_0", "SECRET_VALUE_1", "SECRET_VALUE_2", "SECRET_VALUE_3"] {
                        assert!(!rendered.contains(marker));
                    }
                }
            }
        }
    }
}

#[test]
fn test_control_characters_in_colliding_environment_names_are_escaped() {
    let names = ["APP_FOO\n", "APP_foo\n"];
    let source = EnvConfigSource::from_prefix("APP_");

    for ordered_names in [names, [names[1], names[0]]] {
        let mut context = source_context(SourceLimits::default());
        let error = source
            .load_environment_entries(entries(&ordered_names), &mut context)
            .unwrap_err();
        assert!(matches!(
            &error,
            ConfigError::KeyConflict { path, existing, incoming, .. }
                if path == "foo\n"
                    && existing == "environment variable 'APP_FOO\n'"
                    && incoming == "environment variable 'APP_foo\n'"
        ));
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains("SECRET_VALUE"));
        assert!(rendered.contains(r"foo\n"));
        assert!(rendered.contains(r"APP_FOO\n"));
        assert!(rendered.contains(r"APP_foo\n"));
    }
}

#[test]
fn test_unicode_helpers_borrow_the_original_os_string() {
    let key = OsString::from("APP_Å_LONG_KEY_123456789");
    let value = OsString::from("值-🦀-a-long-valid-UTF8-value-123456789");
    let key_str = EnvConfigSource::env_key_to_str(&key).unwrap();
    let value_str = EnvConfigSource::env_value_to_str(&key, &value).unwrap();

    assert_eq!(key_str, key.to_str().unwrap());
    assert_eq!(value_str, value.to_str().unwrap());
    assert_eq!(key_str.as_ptr(), key.to_str().unwrap().as_ptr());
    assert_eq!(value_str.as_ptr(), value.to_str().unwrap().as_ptr());
}

#[test]
fn test_input_byte_budget_error_is_exact_and_keeps_output_empty() {
    let source = EnvConfigSource::new();
    let key = OsString::from("KEY");
    let value = OsString::from("值");
    let observed = key.to_str().unwrap().len() + value.to_str().unwrap().len();
    let limits = SourceLimits::builder().max_input_bytes(observed - 1).build();
    let mut context = source_context(limits);
    let error = source
        .load_environment_entries([(key, value)], &mut context)
        .expect_err("the UTF-8 input bytes exceed the configured budget");

    assert!(matches!(
        error,
        ConfigError::SourceLimitExceeded {
            kind: crate::source::SourceLimitKind::InputBytes,
            limit,
            observed_at_least,
            ..
        } if limit == observed - 1 && observed_at_least == observed
    ));
    assert!(context.finish().is_empty());
}

#[cfg(unix)]
#[test]
fn test_value_unicode_error_precedes_input_byte_budget() {
    use std::os::unix::ffi::OsStringExt;

    let source = EnvConfigSource::new();
    let invalid_value = OsString::from_vec(vec![0xff]);
    let mut context = source_context(SourceLimits::builder().max_input_bytes(0).build());
    let error = source
        .load_environment_entries([(OsString::from("KEY"), invalid_value)], &mut context)
        .expect_err("invalid Unicode value must be reported before budget exhaustion");

    assert!(error.to_string().contains("value is not valid Unicode"));
    assert!(!matches!(error, ConfigError::SourceLimitExceeded { .. }));
}

#[cfg(unix)]
#[test]
fn test_key_unicode_error_precedes_value_unicode_and_input_budget() {
    use std::os::unix::ffi::OsStringExt;

    let source = EnvConfigSource::new();
    let invalid_key = OsString::from_vec(vec![b'K', 0xff]);
    let invalid_value = OsString::from_vec(vec![0xfe]);
    let mut context = source_context(SourceLimits::builder().max_input_bytes(0).build());
    let error = source
        .load_environment_entries([(invalid_key, invalid_value)], &mut context)
        .expect_err("invalid key must retain priority over invalid value and budget errors");

    assert!(error.to_string().contains("Environment variable key is not valid Unicode"));
    assert!(!error.to_string().contains("value is not valid Unicode"));
    assert!(!matches!(error, ConfigError::SourceLimitExceeded { .. }));
}

#[cfg(unix)]
#[test]
fn test_prefix_filter_skips_invalid_unicode_environment_entry() {
    use std::os::unix::ffi::OsStringExt;

    let mut invalid_key = b"UNRELATED_".to_vec();
    invalid_key.push(0xff);
    let source = EnvConfigSource::from_prefix("APP_");
    let mut context = source_context(SourceLimits::default());

    source
        .load_environment_entries(
            [(OsString::from_vec(invalid_key), OsString::from_vec(vec![0xfe]))],
            &mut context,
        )
        .expect("a non-matching invalid Unicode entry must be filtered out");

    assert!(context.finish().is_empty());
}
