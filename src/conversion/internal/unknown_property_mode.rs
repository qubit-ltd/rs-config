// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Controls whether structured reads reject fields not consumed by Serde.
#[derive(Clone, Copy)]
pub(crate) enum UnknownPropertyMode {
    /// Reject every field not consumed by the target type.
    Reject,
    /// Ignore fields not consumed by the target type.
    Ignore,
}
