// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private utility implementation types.

/// Resource dimensions charged while resolving interpolation.
mod interpolation_resource;
mod interpolation_session;

pub(super) use interpolation_resource::InterpolationResource;
pub(crate) use interpolation_session::InterpolationSession;
