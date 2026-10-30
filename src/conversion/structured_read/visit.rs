// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! One traversal item with its path, depth, and original source index.
//!
//! `path` tracks the current logical configuration location, while
//! `original_index` keeps the outer source collection index unchanged as the
//! walk descends into nested values.

use std::borrow::Cow;

use super::node::SourceLocation;
use super::read_view::ReadView;

pub(super) struct Visit<'a> {
    /// Borrowed logical value at this traversal coordinate.
    pub(super) value: ReadView<'a>,
    /// Original-property coordinate, using typed key/index segments.
    pub(super) location: Option<SourceLocation>,
    /// Diagnostic configuration path; literal dots in keys are not escaped.
    pub(super) path: Cow<'a, str>,
    /// JSON shape depth: the selected root is depth 1 and each child adds 1.
    pub(super) depth: usize,
    /// Key of this object child, if this visit represents one.
    pub(super) key: Option<&'a str>,
    /// First array index on the walk from the root, retained through nesting.
    pub(super) original_index: Option<usize>,
}

impl Visit<'_> {
    /// Returns the retained outer source index used in conversion diagnostics.
    pub(super) fn source_index(&self) -> Option<usize> {
        self.original_index
    }
}
