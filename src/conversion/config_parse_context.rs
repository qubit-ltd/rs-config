// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::ConfigResult;
use crate::options::ReadPolicy;

/// Context passed to [`crate::conversion::FromConfig`] implementations.
pub struct ConfigParseContext<'a> {
    /// The root-relative configuration key.
    key: &'a str,
    /// The read policy used for this parse operation.
    options: &'a ReadPolicy,
    /// The substitution function used for this parse operation.
    substitute: &'a dyn Fn(&str) -> ConfigResult<String>,
}

impl<'a> ConfigParseContext<'a> {
    /// Creates a parsing context.
    ///
    /// # Parameters
    ///
    /// * `key` - The root-relative configuration key.
    /// * `options` - The read policy used for this parse operation.
    /// * `substitute` - The substitution function used for this parse
    ///   operation.
    ///
    /// # Returns
    ///
    /// A new parsing context.
    pub(crate) fn new(
        key: &'a str,
        options: &'a ReadPolicy,
        substitute: &'a dyn Fn(&str) -> ConfigResult<String>,
    ) -> Self {
        Self {
            key,
            options,
            substitute,
        }
    }

    /// Gets the key being parsed.
    ///
    /// # Returns
    ///
    /// The root-relative configuration key.
    #[inline]
    pub fn key(&self) -> &str {
        self.key
    }

    /// Gets the read policy used for this parse operation.
    ///
    /// # Returns
    ///
    /// Read policy selected by the reader.
    #[inline]
    pub fn options(&self) -> &ReadPolicy {
        self.options
    }

    /// Applies variable substitution to a string value.
    ///
    /// # Parameters
    ///
    /// * `value` - The string value to substitute.
    ///
    /// # Returns
    ///
    /// The substituted string value.
    pub(crate) fn substitute_string(
        &self,
        value: &str,
    ) -> ConfigResult<String> {
        (self.substitute)(value)
    }
}
