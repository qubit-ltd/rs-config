/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
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
//! ```rust,ignore
//! use qubit_config::source::{CompositeSource, TomlSource, EnvSource, ConfigSource};
//! use qubit_config::Config;
//!
//! let mut composite = CompositeSource::new();
//! composite.add(TomlSource::from_file("defaults.toml"));
//! composite.add(TomlSource::from_file("config.toml"));
//! composite.add(EnvSource::with_prefix("APP_"));
//!
//! let mut config = Config::new();
//! composite.load(&mut config).unwrap();
//! ```
//!
//! # Author
//!
//! Haixing Hu

use crate::{Config, ConfigResult};

use super::ConfigSource;

/// Configuration source that merges multiple sources in order
///
/// # Author
///
/// Haixing Hu
pub struct CompositeSource {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl CompositeSource {
    /// Creates a new empty `CompositeSource`
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
    pub fn add<S: ConfigSource + 'static>(&mut self, source: S) -> &mut Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Returns the number of sources in this composite
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns `true` if this composite has no sources
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl Default for CompositeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for CompositeSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        for source in &self.sources {
            source.load(config)?;
        }
        Ok(())
    }
}
