// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! One borrowed child yielded during prepared-read traversal.
//!
//! The child's value and optional origin borrow the prepared tree. The origin
//! is present only when source-location tracking is retained for interpolation
//! or when missing-value context needs it.

use super::super::read_view::ReadView;
use super::super::source_location::SourceLocation;
use super::segment::Segment;

/// One child borrowed from a prepared-read tree.
///
/// The traversal segment is a logical path component. An optional origin
/// identifies the original property and typed nested location instead of
/// an arena node ID or a directly reusable configuration lookup key.
pub(crate) struct Child<'a> {
    /// Logical key or collection index used while traversing this child.
    pub(crate) segment: Segment<'a>,
    /// Borrowed view of the child's prepared value.
    pub(crate) value: ReadView<'a>,
    /// Original source location when tracking is retained for interpolation
    /// or missing-value context; its index addresses the property table.
    pub(crate) origin: Option<&'a SourceLocation>,
}
