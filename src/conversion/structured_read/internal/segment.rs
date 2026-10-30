// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! A single key or collection index in a prepared read path.
//!
//! These segments describe the logical dotted configuration path used in
//! diagnostics; they do not encode the typed source location used to identify
//! interpolation overlays.

pub(crate) enum Segment<'a> {
    Key(&'a str),
    Index(usize),
}
