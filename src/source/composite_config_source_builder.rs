// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::CompositeConfigSource;
use super::ConfigSource;
use super::SourceLimits;

/// Builder for [`CompositeConfigSource`].
///
/// Create it with [`CompositeConfigSource::builder`] to add sources and set
/// aggregate [`SourceLimits`] before loading.
#[must_use = "use the returned value"]
pub struct CompositeConfigSourceBuilder {
    /// Sources to apply in insertion order.
    sources: Vec<Box<dyn ConfigSource>>,
    /// Aggregate limits for the completed composite.
    limits: SourceLimits,
}

impl CompositeConfigSourceBuilder {
    /// Creates an empty composite source builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            limits: SourceLimits::default(),
        }
    }

    /// Adds a source to the composite in application order.
    pub fn add_source<S: ConfigSource + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Sets aggregate resource limits for the complete composite load.
    pub fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the configured composite source.
    pub fn build(self) -> CompositeConfigSource {
        CompositeConfigSource {
            sources: self.sources,
            limits: self.limits,
        }
    }
}

impl Default for CompositeConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
