// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Semantic oracles reused by integration tests and fuzz targets.

#[path = "source_semantics/dotenv.rs"]
#[cfg(feature = "env-file")]
pub mod dotenv;
#[path = "source_semantics/properties.rs"]
pub mod properties;
