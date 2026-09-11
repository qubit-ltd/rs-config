// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Owned property wire representation used during deserialization.
mod property_wire_owned;
/// Borrowed property wire representation used during serialization.
mod property_wire_ref;

pub(super) use property_wire_owned::PropertyWireOwned;
pub(super) use property_wire_ref::PropertyWireRef;
