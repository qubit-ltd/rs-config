// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::Config;
use crate::ConfigResult;

/// Trait for configuration sources.
///
/// Implementors return an independent [`Config`] layer. Callers can inspect
/// or compose that layer directly, or apply it atomically with
/// [`Config::merge_from_source`].
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
    /// Loads configuration data into an independent `Config` layer.
    ///
    /// # Parameters
    ///
    /// # Returns
    ///
    /// Returns `Ok(Config)` on success, or a `ConfigError` on failure.
    fn load(&self) -> ConfigResult<Config>;
}
