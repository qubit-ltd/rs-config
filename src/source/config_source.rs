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

    /// Loads configuration data into an already-staged `Config` object.
    ///
    /// Built-in sources override this method to avoid cloning when the caller
    /// has already staged a transaction. Custom implementations can rely on
    /// the default, which delegates to [`Self::load`].
    ///
    /// # Parameters
    ///
    /// * `config` - The staged configuration object to populate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `ConfigError` on failure
    #[inline]
    fn load_into(&self, config: &mut Config) -> ConfigResult<()> {
        self.load(config)
    }
}

/// Runs a source load against a cloned config and commits only on success.
///
/// # Parameters
///
/// * `source` - Source whose non-transactional loader should be staged.
/// * `config` - Target configuration to update atomically.
///
/// # Returns
///
/// Returns `Ok(())` after committing the staged config, or propagates the
/// source error while leaving `config` unchanged.
pub(crate) fn load_transactionally<S>(source: &S, config: &mut Config) -> ConfigResult<()>
where
    S: ConfigSource + ?Sized,
{
    let mut staged = config.clone();
    source.load_into(&mut staged)?;
    *config = staged;
    Ok(())
}
