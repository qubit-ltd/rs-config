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
//! - [`PropertiesConfigSource`]: Loads configuration from Java `.properties` format files
//! - [`TomlConfigSource`]: Loads configuration from TOML format files
//! - [`YamlConfigSource`]: Loads configuration from YAML format files
//! - [`EnvFileConfigSource`]: Loads configuration from `.env` format files
//! - [`EnvConfigSource`]: Loads configuration from system environment variables
//! - [`CompositeConfigSource`]: Merges configuration from multiple sources
//!
//! # Examples
//!
//! ```rust,ignore
//! use qubit_config::{Config, source::{TomlConfigSource, EnvConfigSource, CompositeConfigSource, ConfigSource}};
//!
//! // Load from TOML file with env override
//! let mut composite = CompositeConfigSource::new();
//! composite.add(TomlConfigSource::from_file("config.toml"));
//! composite.add(EnvConfigSource::with_prefix("APP_"));
//!
//! let mut config = Config::new();
//! config.merge_from_source(&composite).unwrap();
//! ```
//!
//! # Author
//!
//! Haixing Hu

mod composite_config_source;
mod config_source;
mod env_config_source;
mod env_file_config_source;
mod properties_config_source;
mod toml_config_source;
mod yaml_config_source;

pub use composite_config_source::CompositeConfigSource;
pub use config_source::ConfigSource;
pub use env_config_source::EnvConfigSource;
pub use env_file_config_source::EnvFileConfigSource;
pub use properties_config_source::PropertiesConfigSource;
pub use toml_config_source::TomlConfigSource;
pub use yaml_config_source::YamlConfigSource;
