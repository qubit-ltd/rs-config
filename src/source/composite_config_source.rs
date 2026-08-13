// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Composite Configuration Source
//!
//! Merges configuration from multiple sources in order.
//!
//! Sources are applied in the order they are added. Later sources override
//! earlier sources for the same key (unless the property is marked as final).
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "toml")]
//! # {
//! use qubit_config::Config;
//! use qubit_config::source::{
//!     CompositeConfigSource, ConfigSource, TomlConfigSource,
//! };
//!
//! let mut composite = CompositeConfigSource::new();
//! let temp_dir = tempfile::tempdir().unwrap();
//! let defaults = temp_dir.path().join("defaults.toml");
//! let override_file = temp_dir.path().join("config.toml");
//! std::fs::write(&defaults, "port = 80\n").unwrap();
//! std::fs::write(&override_file, "port = 8080\n").unwrap();
//! composite.add(TomlConfigSource::from_file(defaults));
//! composite.add(TomlConfigSource::from_file(override_file));
//!
//! let mut config = Config::new();
//! let config = composite.load().unwrap();
//! assert_eq!(config.get::<i64>("port").unwrap(), 8080);
//! # }
//! ```

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use crate::ConfigResult;

/// Configuration source that merges multiple sources in order
pub struct CompositeConfigSource {
    sources: Vec<Box<dyn ConfigSource>>,
    limits: SourceLimits,
}

impl CompositeConfigSource {
    /// Creates a new empty `CompositeConfigSource`.
    ///
    /// # Returns
    ///
    /// An empty composite with no inner sources.
    #[inline]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            limits: SourceLimits::default(),
        }
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
    #[inline]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns `true` if this composite has no sources.
    ///
    /// # Returns
    ///
    /// `true` when [`Self::len`] is zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Applies aggregate resource limits to the complete composite load.
    pub const fn with_limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
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
