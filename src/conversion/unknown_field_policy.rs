// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavior for fields that a structured target does not consume.

/// Controls how structured reads handle unknown fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    /// Reject unknown fields with sorted root-relative paths.
    #[default]
    Reject,
    /// Ignore unconsumed fields after their complete input admission.
    Ignore,
}
