// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! One traversal item with its path, depth, and original source index.

use std::borrow::Cow;

use super::node::SourceLocation;
use super::read_view::ReadView;

pub(super) struct Visit<'a> {
    pub(super) value: ReadView<'a>,
    pub(super) location: Option<SourceLocation>,
    pub(super) path: Cow<'a, str>,
    pub(super) depth: usize,
    pub(super) key: Option<&'a str>,
    pub(super) original_index: Option<usize>,
}

impl Visit<'_> {
    pub(super) fn source_index(&self) -> Option<usize> {
        self.original_index
    }
}
