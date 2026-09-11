// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! A single key or collection index in a prepared read path.

pub(crate) enum Segment<'a> {
    Key(&'a str),
    Index(usize),
}
