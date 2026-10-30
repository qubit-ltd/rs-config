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
    /// Remaining children of this ancestor in the iterative depth-first walk.
    pub(crate) children: ChildIter<'a>,
    /// Logical diagnostic path to the value whose children are being visited.
    pub(crate) path: Cow<'a, str>,
    /// Typed original-source location; key and index segments remain distinct
    /// even where the rendered path uses dots and brackets.
    pub(crate) location: Option<SourceLocation>,
    /// Child depth to assign next; the selected root begins at depth 1.
    pub(crate) depth: usize,
    /// First source array index retained unchanged through nested frames.
    pub(crate) original_index: Option<usize>,
}
