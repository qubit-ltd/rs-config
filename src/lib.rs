/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Management Module
//!
//! Provides flexible configuration management with support for multiple data
//! types and variable substitution.
//!
//! # Author
//!
//! Haixing Hu

mod config;
mod config_error;
mod config_prefix_view;
mod config_property_mut;
mod config_reader;
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
pub use config_prefix_view::ConfigPrefixView;
pub use config_property_mut::ConfigPropertyMut;
pub use config_reader::ConfigReader;
pub use configurable::Configurable;
pub use configured::Configured;
pub use error::{ConfigError, ConfigResult};
pub use property::Property;
pub use source::ConfigSource;
