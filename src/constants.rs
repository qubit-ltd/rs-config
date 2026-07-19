// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Default maximum recursion depth when resolving `${...}` variable references
/// in strings.
///
/// # Returns
///
/// The numeric constant `64`.
pub const DEFAULT_MAX_SUBSTITUTION_DEPTH: usize = 64;

/// Default maximum number of `${...}` placeholder resolutions in one read.
pub const DEFAULT_MAX_SUBSTITUTION_EXPANSIONS: usize = 4_096;

/// Default maximum UTF-8 byte length of one expanded configuration value.
pub const DEFAULT_MAX_SUBSTITUTION_OUTPUT_BYTES: usize = 1_048_576;
