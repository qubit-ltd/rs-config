// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stable classifications for configuration errors.

/// Machine-readable category of a [`crate::ConfigError`].
#[non_exhaustive]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigErrorKind {
    /// A requested property does not exist.
    PropertyNotFound,
    /// A property exists but has no usable value.
    PropertyHasNoValue,
    /// The stored type differs from the exact requested type.
    TypeMismatch,
    /// Converting the stored value failed.
    Conversion,
    /// A lower-level value operation failed.
    Value,
    /// Resolving a variable failed.
    Substitution,
    /// Variable expansion exceeded its configured depth.
    SubstitutionDepthExceeded,
    /// Variable expansion resolved too many placeholders.
    SubstitutionExpansionLimitExceeded,
    /// Variable expansion produced too many UTF-8 bytes.
    SubstitutionOutputTooLarge,
    /// Variable expansion encountered a reference cycle.
    SubstitutionCycle,
    /// Combining configurations failed.
    Merge,
    /// A final property rejected mutation.
    PropertyIsFinal,
    /// A dotted key conflicts with another key shape.
    KeyConflict,
    /// Reading configuration data failed at the I/O layer.
    Io,
    /// Parsing source configuration data failed.
    Parse,
    /// Mapping configuration data into a structured value failed.
    Deserialize,
    /// An uncategorized configuration operation failed.
    Other,
}
