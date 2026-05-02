/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configurable Interface
//!
//! Provides the `Configurable` trait for types to have unified configuration
//! access and change callback interfaces.
//!

use super::Config;

/// Configurable trait
///
/// Types that implement this trait can be configured using `Config`.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, Configurable};
///
/// struct Server { config: Config }
///
/// impl Configurable for Server {
///     fn config(&self) -> &Config {
///         &self.config
///     }
///     fn config_mut(&mut self) -> &mut Config {
///         &mut self.config
///     }
///     fn set_config(&mut self, config: Config) {
///         self.config = config;
///         self.on_config_changed();
///     }
/// }
/// ```
///
/// ```rust
/// use qubit_config::{ConfigResult, ConfigError};
/// ```
///
pub trait Configurable {
    /// Gets a reference to the configuration
    ///
    /// # Returns
    ///
    /// Returns an immutable reference to the configuration
    ///
    fn config(&self) -> &Config;

    /// Gets a mutable reference to the configuration
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the configuration
    ///
    fn config_mut(&mut self) -> &mut Config;

    /// Sets the configuration
    ///
    /// # Parameters
    ///
    /// * `config` - The new configuration
    ///
    /// # Returns
    ///
    /// Nothing.
    ///
    fn set_config(&mut self, config: Config);

    /// Callback after configuration changes
    ///
    /// This method is called when the configuration is modified. Implementors
    /// may override it to run side effects after [`Self::set_config`].
    ///
    /// # Returns
    ///
    /// Nothing.
    ///
    #[inline]
    fn on_config_changed(&mut self) {
        // Default implementation is empty
    }
}
