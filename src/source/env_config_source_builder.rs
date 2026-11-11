// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::EnvConfigOptions;
use super::EnvConfigSource;
use super::SourceLimits;

/// Builder for [`EnvConfigSource`].
///
/// # Examples
///
/// ```
/// use qubit_config::source::EnvConfigSourceBuilder;
///
/// let source = EnvConfigSourceBuilder::new().build();
/// let _ = source;
/// ```
#[must_use = "use the returned value"]
pub struct EnvConfigSourceBuilder {
    /// Key selection and normalization options.
    options: EnvConfigOptions,
    /// Resource limits for one environment scan.
    limits: SourceLimits,
}

impl EnvConfigSourceBuilder {
    /// Creates a builder with default options and limits.
    ///
    /// # Returns
    ///
    /// A builder with default key options and resource limits.
    #[inline]
    pub fn new() -> Self {
        Self {
            options: EnvConfigOptions::default(),
            limits: SourceLimits::default(),
        }
    }

    /// Sets environment key transformation options.
    ///
    /// # Parameters
    ///
    /// * `options` - The key selection and normalization options.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline]
    pub fn options(mut self, options: EnvConfigOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets source resource limits.
    ///
    /// # Parameters
    ///
    /// * `limits` - The limits applied to one environment scan.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline]
    pub fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the configured environment source.
    ///
    /// # Returns
    ///
    /// An environment source with the configured options and limits.
    #[inline]
    pub fn build(self) -> EnvConfigSource {
        EnvConfigSource {
            options: self.options,
            limits: self.limits,
        }
    }
}

impl Default for EnvConfigSourceBuilder {
    /// Creates the default value for this type.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
