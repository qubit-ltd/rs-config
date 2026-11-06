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
/// Input, node, parser-token, property, source-count, semantic-depth, and
/// parser-depth limit failures are reported as [`crate::ConfigError`].
/// Parser-token charges are distinct from semantic node charges. A failed
/// operation does not undo charges accepted by earlier operations.
pub struct SourceLoadContext<'a> {
    /// Owns this source's local budget and borrows active ancestor budgets so
    /// cumulative charges are checked and committed across all scopes.
    session: SourceLoadSession<'a>,
    /// Holds only this source's output properties until its load succeeds;
    /// failed source loads discard this layer, while completed child layers
    /// can be merged into the parent layer.
    layer: Config,
}

impl SourceLoadContext<'static> {
    /// Creates a root context for one source load.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identifier attached to diagnostics from this source.
    /// * `limits` - Resource limits enforced for this source.
    ///
    /// # Returns
    ///
    /// A context with an empty output layer and a fresh accounting session.
    pub(crate) fn new(source_id: String, limits: SourceLimits) -> Self {
        Self {
            session: SourceLoadSession::new(source_id, limits),
            layer: Config::new(),
        }
    }
}

impl<'a> SourceLoadContext<'a> {
    /// Checks parser depth for one input against this source and every
    /// ancestor.
    ///
    /// The root depth is zero. Parser depth is checked independently per input
    /// and is not accumulated across sibling sources.
    ///
    /// # Parameters
    ///
    /// * `depth` - Nesting depth of the input's parser structure.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the depth is within all active limits.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] if the depth exceeds
    /// an active source or aggregate parser-depth limit; the error
    /// identifies the rejecting scope.
    #[inline]
    pub fn check_parser_depth(&self, depth: usize) -> ConfigResult<()> {
        self.session.check_parser_depth(depth)
    }

    /// Charges input bytes to the current source and all aggregate ancestors.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of input bytes to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the charge is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] if the charge
    /// exceeds an active source or aggregate input-byte limit.
    #[inline]
    pub fn consume_input_bytes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_input_bytes(amount)
    }

    /// Charges parsed nodes to the current source and all aggregate ancestors.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of parsed nodes to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the charge is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] if the charge
    /// exceeds an active source or aggregate node limit.
    #[inline]
    pub fn consume_nodes(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_nodes(amount)
    }

    /// Charges parser tokens before syntax-tree construction to this source and
    /// its ancestors.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of parser tokens to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the charge is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] when this charge
    /// would exceed any active source or aggregate parser-token budget. A
    /// rejected charge changes no budget.
    #[inline]
    pub fn consume_parser_tokens(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_parser_tokens(amount)
    }

    /// Charges admitted child sources to the current source and ancestors.
    ///
    /// # Parameters
    ///
    /// * `amount` - Number of child sources to charge.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the charge is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] if the charge
    /// exceeds an active source or aggregate source-count limit.
    #[inline]
    pub fn consume_sources(&mut self, amount: usize) -> ConfigResult<()> {
        self.session.consume_sources(amount)
    }

    /// Sets one output property after checking its semantic depth and charging
    /// one node and one assignment.
    ///
    /// Checks run in this order: semantic depth, node charge, assignment
    /// charge, then key and final-property validation and the write. A rejected
    /// operation does not undo charges accepted before a later check fails,
    /// including the node charge if the assignment charge is rejected. The
    /// executor discards the source layer when loading fails. This is not one
    /// atomic budget transaction across all dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] when the depth,
    /// node, or assignment limit rejects the operation;
    /// [`crate::ConfigError::SourceInvalidKey`] for an invalid key; or
    /// [`crate::ConfigError::PropertyIsFinal`] when the property is final.
    ///
    /// # Parameters
    ///
    /// * `name` - Property key to write in this source's output layer.
    /// * `values` - Value or values assigned to the key.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the property is written successfully.
    /// # Type Parameters
    ///
    /// * `S` - Input value or collection type convertible into a
    ///   [`ValueContainer`].
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

    /// Sets an explicitly typed null value after checking its semantic depth
    /// and charging one node and one assignment.
    ///
    /// Checks run in this order: semantic depth, node charge, assignment
    /// charge, then key and final-property validation and the write. Charges
    /// accepted before a later check fails, including the node charge if the
    /// assignment charge is rejected, are not rolled back. The executor
    /// discards the source layer if loading fails.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] when the depth,
    /// node, or assignment limit rejects the operation;
    /// [`crate::ConfigError::SourceInvalidKey`] for an invalid key; or
    /// [`crate::ConfigError::PropertyIsFinal`] when the property is final.
    ///
    /// # Parameters
    ///
    /// * `name` - Property key to write in this source's output layer.
    /// * `data_type` - Type associated with the explicit null value.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the null property is written successfully.
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
    ///
    /// # Parameters
    ///
    /// * `name` - Existing source-owned property key.
    /// * `is_final` - Whether to lock the property's final marker.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the marker is set or cleared.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::PropertyNotFound`] for a missing property,
    /// [`crate::ConfigError::SourceInvalidKey`] for an invalid name, or
    /// [`crate::ConfigError::PropertyIsFinal`] when an already-final property
    /// is unlocked.
    pub fn set_final(&mut self, name: impl AsRef<str>, is_final: bool) -> ConfigResult<()> {
        let source_id = self.session.source_id().to_owned();
        self.layer
            .set_final(name, is_final)
            .map_err(|error| error.with_assignment_source(&source_id))
    }

    /// Sets descriptive metadata on the source-owned layer.
    ///
    /// # Parameters
    ///
    /// * `description` - Description to attach to this source's output layer.
    #[inline]
    pub fn set_description(&mut self, description: Option<String>) {
        self.layer.set_description(description);
    }

    /// Sets the default read policy on the source-owned layer.
    ///
    /// # Parameters
    ///
    /// * `policy` - Read policy to use when loading this source's output.
    #[inline]
    pub fn set_default_read_policy(&mut self, policy: crate::options::ReadPolicy) {
        self.layer.set_default_read_policy(policy);
    }

    /// Creates a child context while retaining aggregate budget accounting.
    ///
    /// # Parameters
    ///
    /// * `source_id` - Identifier used in child-source diagnostics.
    /// * `limits` - Local limits to apply to the child source.
    ///
    /// # Returns
    ///
    /// A child context sharing this source's aggregate budget chain.
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

    /// Borrows the legacy accounting session for built-in source parsers.
    ///
    /// # Returns
    ///
    /// Mutable access to this source's legacy accounting session.
    #[inline]
    pub(crate) fn session_mut(&mut self) -> &mut SourceLoadSession<'a> {
        &mut self.session
    }

    /// Returns the completed source layer to the crate-owned executor.
    ///
    /// # Returns
    ///
    /// The properties accumulated by this source load.
    #[inline]
    pub(crate) fn finish(self) -> Config {
        self.layer
    }

    /// Merges a completed child layer into this context transactionally.
    ///
    /// # Parameters
    ///
    /// * `layer` - Completed child output to merge into this source layer.
    /// * `source_id` - Child identifier used to annotate merge errors.
    ///
    /// # Returns
    ///
    /// `Ok(())` when all child properties are merged.
    ///
    /// # Errors
    ///
    /// Returns a configuration error annotated with `source_id` if property
    /// merging fails; the operation leaves the parent layer unchanged.
    pub(crate) fn merge_layer(&mut self, layer: Config, source_id: &str) -> ConfigResult<()> {
        self.layer
            .merge_properties(layer)
            .map_err(|error| error.with_merge_source(source_id))
    }

    /// Replaces the layer produced by a built-in source parser.
    ///
    /// # Parameters
    ///
    /// * `layer` - Parsed output layer to store in this context.
    #[inline]
    pub(crate) fn replace_layer(&mut self, layer: Config) {
        self.layer = layer;
    }
}
