// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parsing support for typed configuration reads.

pub use crate::conversion::ConfigParseContext;
pub use crate::conversion::FromConfig;
pub use crate::conversion::IntoConfigDefault;

pub(crate) use crate::helpers::{
    is_effectively_missing,
    is_effectively_missing_interpolated,
    parse_property_from_reader,
    parse_property_from_reader_interpolated,
};
