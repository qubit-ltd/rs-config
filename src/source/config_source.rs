// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::SourceLimits;
use super::SourceLoadSession;
use crate::Config;
use crate::ConfigResult;

/// Trait for configuration sources.
///
/// Implementors return an independent [`Config`] layer. Callers can inspect
/// or compose its properties directly, or apply them atomically with
/// [`Config::merge_properties_from_source`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, source::ConfigSource};
///
/// struct MySource;
///
/// impl ConfigSource for MySource {
///     fn source_id(&self) -> String {
///         "my source".to_string()
///     }
///
///     fn limits(&self) -> qubit_config::source::SourceLimits {
///         qubit_config::source::SourceLimits::default()
///     }
///
///     fn load_with_session(
///         &self,
///         session: &mut qubit_config::source::SourceLoadSession<'_>,
///     ) -> qubit_config::ConfigResult<Config> {
///         session.consume_nodes(1)?;
///         session.consume_properties(1)?;
///         let mut config = Config::new();
///         config.set("key", "value")?;
///         Ok(config)
///     }
/// }
/// ```
pub trait ConfigSource {
    /// Returns the stable identifier used for loading diagnostics.
    fn source_id(&self) -> String;

    /// Returns the local resource limits for one load.
    fn limits(&self) -> SourceLimits;

    /// Loads configuration through an existing local/aggregate budget session.
    fn load_with_session(&self, session: &mut SourceLoadSession<'_>) -> ConfigResult<Config>;

    /// Loads configuration data into an independent `Config` layer.
    ///
    /// # Parameters
    ///
    /// # Returns
    ///
    /// Returns `Ok(Config)` on success, or a `ConfigError` on failure.
    fn load(&self) -> ConfigResult<Config> {
        let mut session = SourceLoadSession::new(self.source_id(), self.limits());
        self.load_with_session(&mut session)
    }
}
