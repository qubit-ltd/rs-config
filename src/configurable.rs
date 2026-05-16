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

use super::{
    Config,
    ConfigResult,
};

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

    /// Gets a mutable reference to the configuration.
    ///
    /// Direct mutations through this reference do not automatically call
    /// [`Self::on_config_changed`]. Use [`Self::update_config`] when a mutation
    /// should trigger the callback once after a successful update.
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

    /// Updates the configuration through a closure and triggers the callback.
    ///
    /// The closure receives the same mutable configuration returned by
    /// [`Self::config_mut`]. The callback is called exactly once after the
    /// closure returns `Ok(())`; if the closure returns an error, the callback
    /// is not called.
    ///
    /// # Parameters
    ///
    /// * `update` - Closure that mutates the configuration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the update succeeds.
    ///
    /// # Errors
    ///
    /// Returns the [`crate::ConfigError`] produced by the closure.
    fn update_config<F>(&mut self, update: F) -> ConfigResult<()>
    where
        Self: Sized,
        F: FnOnce(&mut Config) -> ConfigResult<()>,
    {
        update(self.config_mut())?;
        self.on_config_changed();
        Ok(())
    }

    /// Callback after configuration replacement or controlled updates.
    ///
    /// This method is called by [`Self::set_config`] implementations and by
    /// the default [`Self::update_config`] helper. Direct mutations through
    /// [`Self::config_mut`] are intentionally not observed.
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
