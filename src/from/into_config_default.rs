/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

//! Default value adapters for typed configuration reads.

use qubit_value::IntoValueDefault;

/// Converts a caller-provided fallback value into the typed configuration result.
pub trait IntoConfigDefault<T> {
    /// Converts this fallback value into `T`.
    fn into_config_default(self) -> T;
}

impl<T, D> IntoConfigDefault<T> for D
where
    D: IntoValueDefault<T>,
{
    #[inline]
    fn into_config_default(self) -> T {
        self.into_value_default()
    }
}
