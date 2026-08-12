// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
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
//! - `TomlConfigSource`: Loads configuration from TOML format files when the
//!   `toml` feature is enabled
//! - `YamlConfigSource`: Loads configuration from YAML format files when the
//!   `yaml` feature is enabled
//! - `EnvFileConfigSource`: Loads configuration from `.env` format files when
//!   the `env-file` feature is enabled
//! - [`EnvConfigSource`]: Loads configuration from system environment variables
//! - [`CompositeConfigSource`]: Merges configuration from multiple sources
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
//! // Load from TOML file
//! let mut composite = CompositeConfigSource::new();
//! let temp_dir = tempfile::tempdir().unwrap();
//! let path = temp_dir.path().join("config.toml");
//! std::fs::write(&path, "port = 8080\n").unwrap();
//! composite.add(TomlConfigSource::from_file(path));
//!
//! let mut config = Config::new();
//! config.merge_properties_from_source(&composite).unwrap();
//! assert_eq!(config.get::<i64>("port").unwrap(), 8080);
//! # }
//! ```

mod composite_config_source;
mod config_source;
mod env_config_source;
#[cfg(feature = "env-file")]
mod env_file_config_source;
mod properties_config_source;
mod source_input;
mod source_limit_kind;
mod source_limits;
mod source_load_session;
#[cfg(feature = "toml")]
mod toml_config_source;
#[cfg(feature = "yaml")]
mod yaml_config_source;

pub use composite_config_source::CompositeConfigSource;
pub use config_source::ConfigSource;
pub use config_source::ConfigSourceExt;
pub use env_config_source::EnvConfigOptions;
pub use env_config_source::EnvConfigSource;
#[cfg(feature = "env-file")]
pub use env_file_config_source::EnvFileConfigSource;
pub use properties_config_source::PropertiesConfigSource;
pub use source_limit_kind::SourceLimitKind;
pub use source_limits::DEFAULT_MAX_COMPOSITE_SOURCES;
pub use source_limits::DEFAULT_MAX_SOURCE_DEPTH;
pub use source_limits::DEFAULT_MAX_SOURCE_INPUT_BYTES;
pub use source_limits::DEFAULT_MAX_SOURCE_NODES;
pub use source_limits::DEFAULT_MAX_SOURCE_PROPERTIES;
pub use source_limits::SourceLimits;
pub use source_load_session::SourceLoadContext;
#[cfg(feature = "toml")]
pub use toml_config_source::TomlConfigSource;
#[cfg(feature = "yaml")]
pub use yaml_config_source::YamlConfigSource;
