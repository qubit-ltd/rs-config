// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Saved parent depths for open TOML value containers.

/// Saves the value depth to restore when a TOML container closes.
#[derive(Clone, Copy)]
pub(super) enum Frame {
    /// Parent value depth saved before entering an array.
    Array(
        /// Depth to restore after the array closes.
        usize,
    ),
    /// Parent value depth saved before entering an inline table.
    Inline(
        /// Depth to restore after the inline table closes.
        usize,
    ),
}
