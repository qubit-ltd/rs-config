// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Public configuration-source contract and crate-owned load executor.
// qubit-style: allow multiple-public-types

use super::SourceLimits;
use super::SourceLoadContext;
use crate::Config;
use crate::ConfigResult;

/// A source that writes configuration through a controlled load context.
pub trait ConfigSource {
    /// Returns the stable identifier used for loading diagnostics.
    fn source_id(&self) -> String;

    /// Returns the local resource limits for one load.
    fn limits(&self) -> SourceLimits;

    /// Loads source data into the context-owned independent layer.
    fn load_into(
        &self,
        context: &mut SourceLoadContext<'_>,
    ) -> ConfigResult<()>;

    /// Loads one independent configuration layer through the standard
    /// executor. Prefer importing [`ConfigSourceExt`] in new code.
    fn load(&self) -> ConfigResult<Config> {
        load_source(self)
    }
}

/// Convenience methods for executing a [`ConfigSource`] through the standard
/// budget and transactional layer executor.
pub trait ConfigSourceExt: ConfigSource {
    /// Loads one independent configuration layer.
    fn load(&self) -> ConfigResult<Config> {
        ConfigSource::load(self)
    }
}

impl<S> ConfigSourceExt for S where S: ConfigSource + ?Sized {}

/// Executes a source with a fresh local budget and output layer.
pub(crate) fn load_source<S>(source: &S) -> ConfigResult<Config>
where
    S: ConfigSource + ?Sized,
{
    let mut context =
        SourceLoadContext::new(source.source_id(), source.limits());
    source.load_into(&mut context)?;
    Ok(context.finish())
}
