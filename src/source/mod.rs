/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Source Module
//!
//! Provides various configuration source implementations for loading
//! configuration from different sources such as files, environment variables,
//! etc.
//!
//! # Supported Sources
//!
//! - [`PropertiesConfigSource`]: Loads configuration from Java `.properties`
//!   format files
//! - [`TomlConfigSource`]: Loads configuration from TOML format files
//! - [`YamlConfigSource`]: Loads configuration from YAML format files
//! - [`EnvFileConfigSource`]: Loads configuration from `.env` format files
//! - [`EnvConfigSource`]: Loads configuration from system environment variables
//! - [`CompositeConfigSource`]: Merges configuration from multiple sources
//!
//! # Examples
//!
//! ```rust
//! use qubit_config::Config;
//! use qubit_config::source::{
//!     CompositeConfigSource, ConfigSource, TomlConfigSource,
//! };
//!
//! // Load from TOML file
//! let mut composite = CompositeConfigSource::new();
//! let temp_dir = tempfile::tempdir().unwrap();
//! let path = temp_dir.path().join("config.toml");
//! std::fs::write(&path, "port = 8080\n").unwrap();
//! composite.add(TomlConfigSource::from_file(path));
//!
//! let mut config = Config::new();
//! config.merge_from_source(&composite).unwrap();
//! assert_eq!(config.get::<i64>("port").unwrap(), 8080);
//! ```
//!

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
