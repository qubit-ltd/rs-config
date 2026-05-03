/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
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
///     fn load(&self, config: &mut Config) -> qubit_config::ConfigResult<()> {
///         config.set("key", "value")?;
///         Ok(())
///     }
/// }
/// ```
///
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
