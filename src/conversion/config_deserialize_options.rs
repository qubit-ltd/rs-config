// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Per-read choices for structured configuration deserialization.

use crate::UnknownFieldPolicy;

/// Controls interpolation and unconsumed fields for one structured read.
///
/// Conversion policies and resource limits belong to the reader's ReadPolicy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConfigDeserializeOptions {
    /// Whether to interpolate actual String leaves before target conversion.
    pub interpolate: bool,
    /// Whether fields not consumed by the target cause an error.
    pub unknown_fields: UnknownFieldPolicy,
}
