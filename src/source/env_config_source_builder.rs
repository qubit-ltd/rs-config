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
#[must_use = "use the returned value"]
pub struct EnvConfigSourceBuilder {
    /// Key selection and normalization options.
    options: EnvConfigOptions,
    /// Resource limits for one environment scan.
    limits: SourceLimits,
}

impl EnvConfigSourceBuilder {
    /// Creates a builder with default options and limits.
    pub fn new() -> Self {
        Self {
            options: EnvConfigOptions::default(),
            limits: SourceLimits::default(),
        }
    }

    /// Sets environment key transformation options.
    pub fn options(mut self, options: EnvConfigOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets source resource limits.
    pub fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the configured environment source.
    pub fn build(self) -> EnvConfigSource {
        EnvConfigSource {
            options: self.options,
            limits: self.limits,
        }
    }
}

impl Default for EnvConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
