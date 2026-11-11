// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Sealing trait that exposes the root configuration internally.
mod helpers;
mod sealed;

pub(super) use helpers::get_optional_any_with_options;
pub(super) use helpers::missing_candidates_error;
pub(super) use helpers::optional_converted_value;
pub(crate) use sealed::Sealed;
