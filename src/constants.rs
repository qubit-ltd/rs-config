/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

/// Default maximum recursion depth when resolving `${...}` variable references
/// in strings.
///
/// # Returns
///
/// The numeric constant `64`.
pub const DEFAULT_MAX_SUBSTITUTION_DEPTH: usize = 64;
