// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Diagnostic paths allocate once per visited child, without payload copies.

use std::fmt::Write;

pub(super) fn child_key(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        return key.to_owned();
    }
    let mut result = String::with_capacity(parent.len().saturating_add(key.len()).saturating_add(1));
    result.push_str(parent);
    result.push('.');
    result.push_str(key);
    result
}

pub(super) fn child_index(parent: &str, index: usize) -> String {
    let mut result = String::with_capacity(parent.len().saturating_add(2 + usize::MAX.ilog10() as usize + 1));
    result.push_str(parent);
    write!(&mut result, "[{index}]").expect("writing to String is infallible");
    result
}
