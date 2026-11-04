// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Heap-owned parser state for one active interpolation value.
pub(in crate::utils) struct InterpolationFrame {
    /// Text currently being scanned for placeholders.
    pub(in crate::utils) text: String,
    /// Byte position at which placeholder scanning resumes.
    pub(in crate::utils) search_from: usize,
    /// End of the last placeholder consumed from `text`.
    pub(in crate::utils) last_end: usize,
    /// Output assembled from this value's expanded fragments.
    pub(in crate::utils) output: String,
    /// Variable whose resolution entered this frame, if any.
    pub(in crate::utils) entering_variable: Option<String>,
    /// Whether the value's plain-text and depth checks have been performed.
    pub(in crate::utils) entered: bool,
}

impl InterpolationFrame {
    /// Creates an unentered frame from an owned input string.
    pub(in crate::utils) fn new(text: String, entering_variable: Option<String>) -> Self {
        Self {
            text,
            search_from: 0,
            last_end: 0,
            output: String::new(),
            entering_variable,
            entered: false,
        }
    }
}
