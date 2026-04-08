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
//! Provides flexible configuration management with support for multiple data types and variable substitution.
//!
//! # Author
//!
//! Haixing Hu

mod config;
mod configurable;
mod configured;
mod error;
mod property;
pub mod source;
mod utils;

pub use config::Config;
pub use configurable::Configurable;
pub use configured::Configured;
pub use error::{ConfigError, ConfigResult};
pub use property::Property;
pub use source::ConfigSource;
pub use utils::substitute_variables;
