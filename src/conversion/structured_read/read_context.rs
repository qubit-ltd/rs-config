// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed context shared by direct Serde visitors.
//!
//! The context and its prepared tree are borrowed for the duration of one
//! structured read. Values yielded by the visitors therefore cannot outlive
//! the input or its prepared projection.

use super::prepared_read::PreparedConfigRead;
use std::cell::RefCell;

#[derive(Clone, Copy)]
pub(super) struct ReadContext<'tree> {
    /// Prepared tree and source metadata shared by visitors.
    pub(super) prepared: &'tree PreparedConfigRead<'tree>,
    /// Unknown source paths collected only for rejecting structured reads.
    pub(super) ignored_paths: Option<&'tree RefCell<Vec<String>>>,
}

impl ReadContext<'_> {
    /// Reports whether source locations are needed for interpolation
    /// diagnostics.
    pub(super) fn needs_locations(self) -> bool {
        !self.prepared.overlays.is_empty()
    }
}
