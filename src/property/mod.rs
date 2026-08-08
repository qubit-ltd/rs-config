// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stored configuration properties and guarded mutation.

#[path = "property.rs"]
mod property_impl;

pub use property_impl::Property;

pub use crate::config_property_mut::ConfigPropertyMut;
