// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Diagnostic paths allocate once per visited child, without payload copies.

use std::fmt::Write;

/// Appends one raw object key to an already formatted diagnostic parent.
pub(super) fn child_key(parent: &str, key: &str) -> String {
    let mut result = String::with_capacity(parent.len().saturating_add(key.len()).saturating_add(4));
    result.push_str(parent);
    append_key(&mut result, key);
    result
}

/// Appends one raw object key to an already formatted diagnostic path.
pub(super) fn append_key(path: &mut String, key: &str) {
    let quoted = key.is_empty()
        || key
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '.' | '[' | ']' | '"' | '\\'));
    if quoted {
        let escaped = serde_json::to_string(key).expect("serializing a string is infallible");
        path.push('[');
        path.push_str(&escaped);
        path.push(']');
    } else if path.is_empty() {
        path.push_str(key);
    } else {
        path.push('.');
        path.push_str(key);
    }
}

/// Converts a canonical dotted configuration path into an unambiguous
/// diagnostic path by encoding each original key segment exactly once.
pub(super) fn config_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut result = String::new();
    for key in path.split('.') {
        append_key(&mut result, key);
    }
    result
}

/// Appends one array index to an already formatted diagnostic parent.
pub(super) fn child_index(parent: &str, index: usize) -> String {
    let mut result = String::with_capacity(parent.len().saturating_add(2 + usize::MAX.ilog10() as usize + 1));
    result.push_str(parent);
    write!(&mut result, "[{index}]").expect("writing to String is infallible");
    result
}
