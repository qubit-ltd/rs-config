// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Implementation of a source that combines multiple configuration layers.

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use crate::ConfigResult;

/// Loads configuration layers in insertion order and merges them into one
/// result.
///
/// Sources are loaded in the order supplied through
/// [`CompositeConfigSourceBuilder::add_source`]. Later values override earlier
/// values for the same key, except that a value marked as final cannot be
/// overridden. If any source fails to load or the merge fails,
/// [`ConfigSource::load`] returns an error and does not return a partially
/// merged configuration.
///
/// Each child source's local limits and the composite's aggregate
/// [`SourceLimits`] apply during loading. Property limits count assignments
/// cumulatively across the composite, including assignments later overridden
/// by another source; they do not count only the distinct keys in the final
/// configuration. Configure aggregate limits with
/// [`CompositeConfigSourceBuilder::limits`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigError;
/// use qubit_config::source::{
///     CompositeConfigSource, ConfigSource, PropertiesConfigSource, SourceLimits,
/// };
///
/// let source = CompositeConfigSource::builder()
///     .add_source(PropertiesConfigSource::from_content("port=8080"))
///     .add_source(PropertiesConfigSource::from_content("port=9090"))
///     .build();
/// let config = source.load()?;
/// assert_eq!(config.get::<u16>("port")?, 9090);
///
/// let limited = CompositeConfigSource::builder()
///     .limits(SourceLimits::builder().max_properties(1).build())
///     .add_source(PropertiesConfigSource::from_content("a=1"))
///     .add_source(PropertiesConfigSource::from_content("b=2"))
///     .build();
/// assert!(matches!(
///     limited.load(),
///     Err(ConfigError::SourceLimitExceeded { .. })
/// ));
/// # Ok::<(), ConfigError>(())
/// ```
pub struct CompositeConfigSource {
    /// Sources applied in insertion order.
    sources: Vec<Box<dyn ConfigSource>>,
    /// Aggregate limits applied to the composite load.
    limits: SourceLimits,
}

impl CompositeConfigSource {
    /// Creates a builder initialized with an empty composite source.
    #[inline]
    pub fn builder() -> CompositeConfigSourceBuilder {
        CompositeConfigSourceBuilder::new()
    }

    /// Creates a new empty `CompositeConfigSource`.
    ///
    /// # Returns
    ///
    /// An empty composite with no inner sources.
    #[inline]
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Adds a configuration source
    ///
    /// Sources are applied in the order they are added. Later sources override
    /// earlier sources for the same key.
    ///
    /// # Parameters
    ///
    /// * `source` - The configuration source to add
    ///
    /// # Returns
    ///
    /// `self` for method chaining.
    #[inline]
    pub fn add<S: ConfigSource + 'static>(&mut self, source: S) -> &mut Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Returns the number of sources in this composite.
    ///
    /// # Returns
    ///
    /// The length of the internal source list.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns `true` if this composite has no sources.
    ///
    /// # Returns
    ///
    /// `true` when [`Self::len`] is zero.
    #[inline(always)]
    #[must_use = "use the returned value"]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

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

impl Default for CompositeConfigSource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for CompositeConfigSource {
    fn source_id(&self) -> String {
        "composite configuration source".to_string()
    }

    fn limits(&self) -> SourceLimits {
        self.limits
    }

    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        for source in &self.sources {
            context.consume_sources(1)?;
            let layer = {
                let mut child = context.child(source.source_id(), source.limits());
                source.load_into(&mut child)?;
                child.finish()
            };
            context.merge_layer(layer)?;
        }
        Ok(())
    }
}
