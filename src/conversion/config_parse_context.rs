// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::ConfigResult;
use crate::conversion::internal::PreparedScalar;
use crate::options::ReadPolicy;

/// Context passed to [`crate::conversion::FromConfig`] implementations.
pub struct ConfigParseContext<'a> {
    /// The root-relative configuration key.
    key: &'a str,
    /// The read policy used for this parse operation.
    options: &'a ReadPolicy,
    /// The substitution function used for this parse operation.
    substitute: &'a dyn Fn(&str) -> ConfigResult<String>,
    /// Whether string values should be interpolated before conversion.
    interpolate: bool,
    /// Optional scalar string prepared for the current property.
    prepared_scalar: Option<PreparedScalar<'a>>,
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
    /// * `interpolate` - Whether string values should be interpolated before
    ///   conversion.
    /// * `prepared_scalar` - Optional borrowed scalar string belonging to the
    ///   property being parsed.
    ///
    /// # Returns
    ///
    /// A new parsing context.
    pub(crate) fn new(
        key: &'a str,
        options: &'a ReadPolicy,
        substitute: &'a dyn Fn(&str) -> ConfigResult<String>,
        interpolate: bool,
        prepared_scalar: Option<&'a str>,
    ) -> Self {
        Self {
            key,
            options,
            substitute,
            interpolate,
            prepared_scalar: prepared_scalar.map(PreparedScalar::new),
        }
    }

    /// Gets the key being parsed.
    ///
    /// # Returns
    ///
    /// The root-relative configuration key.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub fn key(&self) -> &str {
        self.key
    }

    /// Gets the read policy used for this parse operation.
    ///
    /// # Returns
    ///
    /// Read policy selected by the reader.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub fn options(&self) -> &ReadPolicy {
        self.options
    }

    /// Reports whether string values are interpolated before conversion.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub const fn interpolates(&self) -> bool {
        self.interpolate
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
    pub fn substitute_string(&self, value: &str) -> ConfigResult<String> {
        if let Some(prepared) = self.prepared_scalar.as_ref()
            && prepared.is_raw(value)
        {
            return prepared.text(self.interpolate, self.substitute).map(str::to_owned);
        }
        (self.substitute)(value)
    }

    /// Borrows the current scalar's original or prepared text when it is a
    /// string.
    pub(crate) fn prepared_scalar_text(&self) -> ConfigResult<Option<&str>> {
        self.prepared_scalar
            .as_ref()
            .map(|prepared| prepared.text(self.interpolate, self.substitute))
            .transpose()
    }
}
