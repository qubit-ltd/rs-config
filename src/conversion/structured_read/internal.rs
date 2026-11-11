// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private implementation types shared by structured reads.

#[path = "internal/child.rs"]
pub(super) mod child;
#[path = "internal/child_iter.rs"]
pub(super) mod child_iter;
#[path = "internal/counting_writer.rs"]
pub(super) mod counting_writer;
#[path = "internal/frame.rs"]
pub(super) mod frame;
#[path = "internal/index_capacity.rs"]
pub(super) mod index_capacity;
#[path = "internal/merge_entries.rs"]
pub(super) mod merge_entries;
#[path = "internal/ordered_string_map_entries.rs"]
pub(super) mod ordered_string_map_entries;
#[path = "internal/segment.rs"]
pub(super) mod segment;
#[path = "internal/sequence_values.rs"]
pub(super) mod sequence_values;
#[path = "internal/source_segment.rs"]
pub(super) mod source_segment;
