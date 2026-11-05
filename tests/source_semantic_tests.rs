// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fixed semantic matrix shared with the parser fuzz targets.

mod support;

fn exercise_selector_matrix(mut check: impl FnMut(&[u8])) {
    check(&[]);

    for selector in u8::MIN..=u8::MAX {
        check(&[selector]);

        let mut maximum_payload = Vec::with_capacity(65);
        maximum_payload.push(selector);
        maximum_payload.extend(std::iter::repeat_n(selector, 64));
        check(&maximum_payload);

        let mut incrementing_payload = Vec::with_capacity(65);
        incrementing_payload.push(selector);
        incrementing_payload.extend(0..64);
        check(&incrementing_payload);
    }
}

#[test]
fn test_properties_semantics_hold_for_fixed_selector_and_payload_matrix() {
    exercise_selector_matrix(support::source_semantics::properties::check);
}

#[cfg(feature = "env-file")]
#[test]
fn test_dotenv_semantics_hold_for_fixed_selector_and_payload_matrix() {
    exercise_selector_matrix(support::source_semantics::dotenv::check);
}
