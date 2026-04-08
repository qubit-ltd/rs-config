/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Source Module
//!
//! Provides various configuration source implementations for loading configuration
//! from different sources such as files, environment variables, etc.
//!
//! # Supported Sources
//!
//! - [`PropertiesSource`]: Loads configuration from Java `.properties` format files
//! - [`TomlSource`]: Loads configuration from TOML format files
//! - [`YamlSource`]: Loads configuration from YAML format files
//! - [`EnvFileSource`]: Loads configuration from `.env` format files
//! - [`EnvSource`]: Loads configuration from system environment variables
//! - [`CompositeSource`]: Merges configuration from multiple sources
//!
//! # Examples
//!
//! ```rust,ignore
//! use qubit_config::{Config, source::{TomlSource, EnvSource, CompositeSource, ConfigSource}};
//!
//! // Load from TOML file with env override
//! let mut composite = CompositeSource::new();
//! composite.add(TomlSource::from_file("config.toml"));
//! composite.add(EnvSource::with_prefix("APP_"));
//!
//! let mut config = Config::new();
//! config.merge_from_source(&composite).unwrap();
//! ```
//!
//! # Author
//!
//! Haixing Hu

mod composite;
mod env_file_source;
mod env_source;
mod properties_source;
mod toml_source;
mod yaml_source;

pub use composite::CompositeSource;
pub use env_file_source::EnvFileSource;
pub use env_source::EnvSource;
pub use properties_source::PropertiesSource;
pub use toml_source::TomlSource;
pub use yaml_source::YamlSource;

use crate::{Config, ConfigResult};

/// Trait for configuration sources
///
/// Implementors of this trait can load configuration data and populate a `Config` object.
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::{Config, source::ConfigSource};
///
/// struct MySource;
///
/// impl ConfigSource for MySource {
///     fn load(&self, config: &mut Config) -> qubit_config::ConfigResult<()> {
///         config.set("key", "value")?;
///         Ok(())
///     }
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
pub trait ConfigSource {
    /// Loads configuration data into the provided `Config` object
    ///
    /// # Parameters
    ///
    /// * `config` - The configuration object to populate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `ConfigError` on failure
    fn load(&self, config: &mut Config) -> ConfigResult<()>;
}
