// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! One borrowed child yielded during prepared-read traversal.

use super::super::read_view::ReadView;
use super::super::source_location::SourceLocation;
use super::segment::Segment;

pub(crate) struct Child<'a> {
    pub(crate) segment: Segment<'a>,
    pub(crate) value: ReadView<'a>,
    pub(crate) origin: Option<&'a SourceLocation>,
}
