// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::OnceCell;

use crate::ConfigResult;

/// Lazily stores one scalar's successful interpolation result.
///
/// A successful result is cached and returned by later reads. Errors are not
/// cached, so a later read retries substitution and can succeed if its inputs
/// have changed.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the source text borrowed by this prepared scalar.
pub(crate) struct PreparedScalar<'a> {
    /// Original scalar text borrowed from the source.
    raw: &'a str,
    /// Successful interpolation result, initialized lazily and cached.
    expanded: OnceCell<String>,
}

impl<'a> PreparedScalar<'a> {
    /// Creates a prepared scalar that borrows its original text.
    ///
    /// # Parameters
    ///
    /// * `raw` - Source text retained for reads that do not interpolate.
    ///
    /// # Returns
    ///
    /// A prepared scalar that borrows `raw` and has no cached result.
    #[must_use]
    #[inline]
    pub(crate) fn new(raw: &'a str) -> Self {
        Self {
            raw,
            expanded: OnceCell::new(),
        }
    }

    /// Returns original or interpolated text, borrowed until this scalar drops.
    ///
    /// When interpolation is enabled, successful substitutions are cached;
    /// failures leave the cache empty and may be retried on a later call.
    ///
    /// # Parameters
    ///
    /// * `interpolate` - Whether to resolve placeholders in the source text.
    /// * `substitute` - Operation that expands the source text when requested.
    ///
    /// # Returns
    ///
    /// Original text when interpolation is disabled, or the successful
    /// interpolation result borrowed from this scalar's cache.
    ///
    /// # Errors
    ///
    /// Returns the substitution error when interpolation fails. The failure
    /// is not cached, so a later call retries the operation.
    pub(crate) fn text(
        &self,
        interpolate: bool,
        substitute: &dyn Fn(&str) -> ConfigResult<String>,
    ) -> ConfigResult<&str> {
        if !interpolate {
            return Ok(self.raw);
        }
        if let Some(value) = self.expanded.get() {
            return Ok(value);
        }
        let value = substitute(self.raw)?;
        let _ = self.expanded.set(value);
        Ok(self.expanded.get().expect("the expanded scalar was just set"))
    }

    /// Returns whether `value` has the same contents as the original text.
    ///
    /// This compares string contents, not pointer identity.
    ///
    /// # Parameters
    ///
    /// * `value` - Candidate text to compare with the original source.
    ///
    /// # Returns
    ///
    /// `true` when `value` has the same contents as the original text.
    #[must_use]
    #[inline]
    pub(crate) fn is_raw(&self, value: &str) -> bool {
        self.raw == value
    }
}
