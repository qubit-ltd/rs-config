/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use crate::ConfigResult;
use crate::options::ConfigReadOptions;

/// Context passed to [`crate::from::FromConfig`] implementations.
///
/// # Author
///
/// Haixing Hu
pub struct ConfigParseContext<'a> {
    /// The root-relative configuration key.
    key: &'a str,
    /// The read options used for this parse operation.
    options: &'a ConfigReadOptions,
    /// The substitution function used for this parse operation.
    substitute: &'a dyn Fn(&str) -> ConfigResult<String>,
}

impl<'a> ConfigParseContext<'a> {
    /// Creates a parsing context.
    ///
    /// # Parameters
    ///
    /// * `key` - The root-relative configuration key.
    /// * `options` - The read options used for this parse operation.
    /// * `substitute` - The substitution function used for this parse operation.
    ///
    /// # Returns
    ///
    /// A new parsing context.
    pub(crate) fn new(
        key: &'a str,
        options: &'a ConfigReadOptions,
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

    /// Gets the read options used for this parse operation.
    ///
    /// # Returns
    ///
    /// Read options selected by the field or reader.
    #[inline]
    pub fn options(&self) -> &ConfigReadOptions {
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
    pub(crate) fn substitute_string(&self, value: &str) -> ConfigResult<String> {
        (self.substitute)(value)
    }
}
