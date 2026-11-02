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
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::CompositeConfigSource;
/// use qubit_config::source::ConfigSource;
/// use qubit_config::source::PropertiesConfigSource;
///
/// let source = CompositeConfigSource::builder()
///     .add_source(PropertiesConfigSource::from_content("server.port=8080"))
///     .add_source(PropertiesConfigSource::from_content("server.port=9090"))
///     .build();
/// let config = source.load().unwrap();
/// assert_eq!(config.get::<u16>("server.port").unwrap(), 9090);
/// ```
#[must_use = "use the returned value"]
pub struct CompositeConfigSourceBuilder {
    /// Sources to apply in insertion order.
    sources: Vec<Box<dyn ConfigSource>>,
    /// Aggregate limits for the completed composite.
    limits: SourceLimits,
    /// Optional diagnostic label for this composite.
    label: Option<String>,
}

impl CompositeConfigSourceBuilder {
    /// Creates an empty composite source builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            limits: SourceLimits::default(),
            label: None,
        }
    }

    /// Adds a source to the composite in application order.
    pub fn add_source<S: ConfigSource + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Sets a content-free diagnostic label for this composite source.
    ///
    /// A non-empty label is used verbatim as the source identifier in errors
    /// and aggregate budget diagnostics. An empty label restores the default
    /// identifier. The last call wins. Labels need not be unique. Do not
    /// include secrets in labels.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        self.label = if label.is_empty() { None } else { Some(label) };
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
            label: self.label,
        }
    }
}

impl Default for CompositeConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
