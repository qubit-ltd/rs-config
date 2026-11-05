// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Collection kinds tracked while scanning YAML tokens.

/// Identifies an open YAML collection that must be paired with its closing
/// token.
///
/// Block mappings and block sequences share [`Block`](Self::Block), because
/// both are closed by `BlockEnd`. Flow mappings and sequences retain distinct
/// variants so their respective closing tokens can be checked.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CollectionFrame {
    /// An open block mapping or block sequence, closed by `BlockEnd`.
    Block,
    /// An open flow mapping, closed by `FlowMappingEnd`.
    FlowMapping,
    /// An open flow sequence, closed by `FlowSequenceEnd`.
    FlowSequence,
}
