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
fn three_colliding_names_choose_same_pair_for_every_permutation() {
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
fn conflict_selection_uses_smallest_normalized_key_and_redacts_values() {
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
