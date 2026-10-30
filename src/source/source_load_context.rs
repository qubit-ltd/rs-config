// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_datatype::DataType;
use qubit_value::ValueContainer;

use super::SourceLimits;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigResult;

/// Controlled output and accounting context supplied to a configuration source.
///
/// The executor creates this context and passes it to
/// [`ConfigSource::load_into`](super::ConfigSource::load_into)
/// during [`ConfigSource::load`](super::ConfigSource::load). It owns an
/// independent layer that is returned only after the source succeeds. If a
/// source returns an error, propagate it; the partial layer is discarded.
///
/// `set` and `set_null` each charge one node and one property assignment
/// themselves. Call [`consume_nodes`](Self::consume_nodes) only for additional
/// parser or structural work, not for properties written with those methods.
/// Each charge is checked atomically against this source and all aggregate
/// ancestors before being committed. Earlier accepted charges are not rolled
/// back if a later operation fails, so this is not an all-or-nothing budget
/// transaction for the complete source load.
///
/// # Examples
///
/// A source receives this context from the executor. Charge its input before
/// writing properties; `set` already charges one node and property assignment,
/// so do not call `consume_nodes` again for that property.
///
/// ```rust
/// use qubit_config::ConfigResult;
/// use qubit_config::source::ConfigSource;
/// use qubit_config::source::SourceLimits;
/// use qubit_config::source::SourceLoadContext;
///
/// struct MemorySource;
///
/// impl ConfigSource for MemorySource {
///     fn source_id(&self) -> String {
///         "example:<memory>".to_owned()
///     }
///
///     fn limits(&self) -> SourceLimits {
///         SourceLimits::default()
///     }
///
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
///
/// Input, node, property, source-count, and nesting-depth limit failures are
/// reported as [`crate::ConfigError`]. A failed operation does not undo charges
/// accepted by earlier operations.
pub struct SourceLoadContext<'a> {
    session: SourceLoadSession<'a>,
    layer: Config,
}

impl SourceLoadContext<'static> {
    /// Creates a root context for one source load.
    pub(crate) fn new(source_id: String, limits: SourceLimits) -> Self {
        Self {
            session: SourceLoadSession::new(source_id, limits),
            layer: Config::new(),
        }
    }
}

impl<'a> SourceLoadContext<'a> {
    /// Charges input bytes to the current source and all aggregate ancestors.
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_input_bytes(amount)
    }

    /// Charges parsed nodes to the current source and all aggregate ancestors.
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_nodes(amount)
    }

    /// Charges admitted child sources to the current source and ancestors.
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_sources(amount)
    }

    /// Sets one output property after validating and charging the assignment.
    pub fn set<S>(&mut self, name: impl AsRef<str>, values: S) -> ConfigResult<()>
    where
        S: Into<ValueContainer>,
    {
        let name = name.as_ref();
        self.session.check_depth(name.split('.').count())?;
        self.session.consume_nodes(1)?;
        self.session.consume_properties(1)?;
        let source_id = self.session.source_id().to_owned();
        self.layer
            .set(name, values)
            .map_err(|error| error.with_assignment_source(&source_id))
    }

    /// Sets an explicitly typed null value after charging the assignment.
    pub fn set_null(&mut self, name: impl AsRef<str>, data_type: DataType) -> ConfigResult<()> {
        let name = name.as_ref();
        self.session.check_depth(name.split('.').count())?;
        self.session.consume_nodes(1)?;
        self.session.consume_properties(1)?;
        let source_id = self.session.source_id().to_owned();
        self.layer
            .set_null(name, data_type)
            .map_err(|error| error.with_assignment_source(&source_id))
    }

    /// Marks an existing source-owned property as final, or clears its final
    /// marker when it has not already been locked.
    ///
    /// Call this after [`set`](Self::set) or [`set_null`](Self::set_null) has
    /// created the property. Repeating the current value is idempotent, while
    /// changing `true` back to `false` is rejected. Missing properties return
    /// [`crate::ConfigError::PropertyNotFound`], and invalid names return
    /// [`crate::ConfigError::SourceInvalidKey`] with this source's identifier.
    /// This changes metadata only and does not charge the source budget
    /// again.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::ConfigResult;
    /// use qubit_config::source::{ConfigSource, SourceLimits, SourceLoadContext};
    ///
    /// struct Defaults;
    /// impl ConfigSource for Defaults {
    ///     fn source_id(&self) -> String { "defaults".to_owned() }
    ///     fn limits(&self) -> SourceLimits { SourceLimits::default() }
    ///     fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
    ///         context.set("service.port", 8080_u16)?;
    ///         context.set_final("service.port", true)
    ///     }
    /// }
    ///
    /// let config = Defaults.load()?;
    /// assert!(config.get_property("service.port")?.expect("property exists").is_final());
    /// # Ok::<(), qubit_config::ConfigError>(())
    /// ```
    pub fn set_final(&mut self, name: impl AsRef<str>, is_final: bool) -> ConfigResult<()> {
        let source_id = self.session.source_id().to_owned();
        self.layer
            .set_final(name, is_final)
            .map_err(|error| error.with_assignment_source(&source_id))
    }

    /// Sets descriptive metadata on the source-owned layer.
    pub fn set_description(&mut self, description: Option<String>) {
        self.layer.set_description(description);
    }

    /// Sets the default read policy on the source-owned layer.
    pub fn set_default_read_policy(&mut self, policy: crate::options::ReadPolicy) {
        self.layer.set_default_read_policy(policy);
    }

    /// Returns the completed source layer to the crate-owned executor.
    pub(crate) fn finish(self) -> Config {
        self.layer
    }

    /// Creates a child context while retaining aggregate budget accounting.
    pub(crate) fn child<'child>(
        &'child mut self,
        source_id: String,
        limits: SourceLimits,
    ) -> SourceLoadContext<'child> {
        SourceLoadContext {
            session: self.session.child(source_id, limits),
            layer: Config::new(),
        }
    }

    /// Merges a completed child layer into this context transactionally.
    pub(crate) fn merge_layer(&mut self, layer: Config) -> ConfigResult<()> {
        self.layer.merge_properties(layer)
    }

    /// Borrows the legacy accounting session for built-in source parsers.
    pub(crate) fn session_mut(&mut self) -> &mut SourceLoadSession<'a> {
        &mut self.session
    }

    /// Replaces the layer produced by a built-in source parser.
    pub(crate) fn replace_layer(&mut self, layer: Config) {
        self.layer = layer;
    }
}
