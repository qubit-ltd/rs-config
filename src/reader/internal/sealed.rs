// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Config;

/// Internal capabilities implemented only by readers owned by this crate.
pub(crate) trait Sealed {
    /// Returns the root configuration backing this reader.
    #[must_use = "the root configuration must be used"]
    fn root_config(&self) -> &Config;
}
