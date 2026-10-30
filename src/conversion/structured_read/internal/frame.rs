// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! One ancestor frame retained by iterative structured-read traversal.
//!
//! Frames keep traversal state on the heap rather than recursing on the Rust
//! call stack. `original_index` carries the outer source collection index
//! through nested child traversal for diagnostics.

use std::borrow::Cow;

use super::super::source_location::SourceLocation;
use super::child_iter::ChildIter;

pub(crate) struct Frame<'a> {
    pub(crate) children: ChildIter<'a>,
    pub(crate) path: Cow<'a, str>,
    pub(crate) location: Option<SourceLocation>,
    pub(crate) depth: usize,
    pub(crate) original_index: Option<usize>,
}
