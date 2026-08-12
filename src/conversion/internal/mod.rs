// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal serde access adapters for configuration values.

mod config_enum_access;
mod config_map_access;
mod config_scalar_seq_access;
mod config_seq_access;
mod config_variant_access;

pub(super) use config_enum_access::ConfigEnumAccess;
pub(super) use config_map_access::ConfigMapAccess;
pub(super) use config_scalar_seq_access::ConfigScalarSeqAccess;
pub(super) use config_scalar_seq_access::admit_scalar_items;
pub(super) use config_seq_access::ConfigSeqAccess;
