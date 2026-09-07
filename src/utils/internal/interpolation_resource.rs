// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Resource dimensions consumed while one interpolation is evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InterpolationResource {
    /// One resolved placeholder.
    Expansions,
    /// UTF-8 bytes present in an intermediate interpolation result.
    OutputBytes,
}
