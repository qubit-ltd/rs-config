// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed context shared by direct Serde visitors.

use super::prepared_read::PreparedConfigRead;

#[derive(Clone, Copy)]
pub(super) struct ReadContext<'tree> {
    /// Prepared tree and source metadata shared by visitors.
    pub(super) prepared: &'tree PreparedConfigRead<'tree>,
}

impl ReadContext<'_> {
    /// Reports whether source locations are needed for interpolation
    /// diagnostics.
    pub(super) fn needs_locations(self) -> bool {
        !self.prepared.overlays.is_empty()
    }
}
