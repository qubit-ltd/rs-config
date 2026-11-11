// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Public configuration-source contract and crate-owned load executor.

use super::SourceLimits;
use super::SourceLoadContext;
use crate::Config;
use crate::ConfigResult;

/// A source that writes configuration through a controlled load context.
///
/// Library-managed loading creates the context and an independent output
/// layer, then calls [`load_into`](Self::load_into), bypassing an overridden
/// [`load`](Self::load) method. Direct calls to an overridden `load` are the
/// implementation's responsibility; the default `load` uses the standard
/// executor. The context is supplied by the executor;
/// source implementations must report input bytes and any extra parser nodes
/// before doing that work. `set` and `set_null` account for their own output
/// node and property assignment.
///
/// A failed `load_into` result is propagated and its layer is not returned.
/// Each resource charge is atomic across the local and aggregate budgets, but
/// charges accepted earlier in the same failed load are not rolled back.
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigReader;
/// use qubit_config::source::ConfigSource;
/// use qubit_config::source::SourceLimits;
/// use qubit_config::source::SourceLoadContext;
/// use qubit_config::ConfigResult;
///
/// struct MemorySource;
/// impl ConfigSource for MemorySource {
///     fn source_id(&self) -> String { "example:<memory>".to_owned() }
///     fn limits(&self) -> SourceLimits { SourceLimits::default() }
///     fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
///         context.consume_input_bytes("port=8080".len())?;
///         context.set("server.port", 8080_u16)
///     }
/// }
///
/// let config = MemorySource.load()?;
/// assert_eq!(config.get::<u16>("server.port")?, 8080);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
pub trait ConfigSource {
    /// Returns the stable identifier used for loading diagnostics.
    ///
    /// # Returns
    ///
    /// A stable source identifier.
    #[must_use = "use the returned source identifier"]
    fn source_id(&self) -> String;

    /// Returns the local resource limits for one load.
    ///
    /// # Returns
    ///
    /// The limits applied to this source during one load.
    fn limits(&self) -> SourceLimits;

    /// Loads source data into the context-owned independent layer.
    ///
    /// # Parameters
    ///
    /// * `context` - The executor-owned context for reporting input use and
    ///   writing this source's independent output layer.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the source is loaded successfully.
    ///
    /// # Errors
    ///
    /// Returns an error when parsing, resource accounting, or writing the
    ///   source data fails.
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()>;

    /// Loads one independent configuration layer through the standard
    /// budget and transactional executor by default.
    ///
    /// Library-managed entry points call [`load_into`](Self::load_into)
    /// through that executor and do not dispatch to an override of this
    /// method. An implementation that overrides `load` is responsible for
    /// behavior of direct calls to that override.
    ///
    /// # Returns
    ///
    /// The configuration layer produced by this source.
    ///
    /// # Errors
    ///
    /// Returns an error if source loading or resource accounting fails.
    #[inline]
    fn load(&self) -> ConfigResult<Config> {
        load_source(self)
    }
}

/// Executes a source with a fresh local budget and output layer.
///
/// # Type Parameters
///
/// * `S` - Source type whose configuration data is loaded.
///
/// # Parameters
///
/// * `source` - Source whose identifier, limits, and data populate the output
///   layer.
///
/// # Returns
///
/// The completed configuration layer produced by the source.
///
/// # Errors
///
/// Returns an error reported by the source or its resource accounting.
pub(crate) fn load_source<S>(source: &S) -> ConfigResult<Config>
where
    S: ConfigSource + ?Sized,
{
    let mut context = SourceLoadContext::new(source.source_id(), source.limits());
    source.load_into(&mut context)?;
    Ok(context.finish())
}
