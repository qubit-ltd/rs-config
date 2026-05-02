/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

//! Ergonomic configuration key argument adapters.

/// Provides borrowed access to a configuration key argument.
pub trait ConfigName {
    /// Invokes `operation` with this argument as a string slice.
    fn with_config_name<R>(self, operation: impl FnOnce(&str) -> R) -> R;
}

impl ConfigName for &str {
    #[inline]
    fn with_config_name<R>(self, operation: impl FnOnce(&str) -> R) -> R {
        operation(self)
    }
}

impl ConfigName for String {
    #[inline]
    fn with_config_name<R>(self, operation: impl FnOnce(&str) -> R) -> R {
        operation(self.as_str())
    }
}

impl ConfigName for &String {
    #[inline]
    fn with_config_name<R>(self, operation: impl FnOnce(&str) -> R) -> R {
        operation(self.as_str())
    }
}
