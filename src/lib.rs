/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Management Module
//!
//! Provides flexible configuration management with support for multiple data
//! types and variable substitution.
//!

mod config;
mod config_deserialize_error;
mod config_error;
mod config_name;
mod config_names;
mod config_prefix_view;
mod config_property_mut;
mod config_reader;
mod config_value_deserializer;
mod configurable;
mod configured;
mod constants;
mod error;
pub mod field;
pub mod from;
pub mod options;
mod property;
pub mod source;
mod utils;

pub use config::Config;
pub use config_name::ConfigName;
pub use config_names::ConfigNames;
pub use config_prefix_view::ConfigPrefixView;
pub use config_property_mut::ConfigPropertyMut;
pub use config_reader::ConfigReader;
pub use configurable::Configurable;
pub use configured::Configured;
pub use error::{ConfigError, ConfigResult};
pub use property::Property;
pub use source::ConfigSource;
