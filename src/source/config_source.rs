// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{
    Config,
    ConfigResult,
};

/// Trait for configuration sources
///
/// Implementors of this trait can load configuration data and populate a
/// [`Config`] object.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, source::ConfigSource};
///
/// struct MySource;
///
/// impl ConfigSource for MySource {
///     fn load(&self) -> qubit_config::ConfigResult<Config> {
///         let mut config = Config::new();
///         config.set("key", "value")?;
///         Ok(config)
///     }
/// }
/// ```
pub trait ConfigSource {
    /// Loads configuration data into an independent `Config` object.
    ///
    /// # Parameters
    ///
    /// # Returns
    ///
    /// Returns `Ok(Config)` on success, or a `ConfigError` on failure.
    fn load(&self) -> ConfigResult<Config>;
}
