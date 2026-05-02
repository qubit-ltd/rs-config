/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
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
//! use qubit_config::source::{
//!     CompositeConfigSource, ConfigSource, TomlConfigSource,
//! };
//! use qubit_config::Config;
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
//! composite.load(&mut config).unwrap();
//! assert_eq!(config.get::<i64>("port").unwrap(), 8080);
//! ```
//!

use crate::{Config, ConfigResult};

use super::ConfigSource;

/// Configuration source that merges multiple sources in order
///
pub struct CompositeConfigSource {
    sources: Vec<Box<dyn ConfigSource>>,
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
}

impl Default for CompositeConfigSource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for CompositeConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        for source in &self.sources {
            source.load(config)?;
        }
        Ok(())
    }
}
