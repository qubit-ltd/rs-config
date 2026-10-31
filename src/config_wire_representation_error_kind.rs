// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Safe category for a configuration wire representation failure.
///
/// The category intentionally omits the rejected value and Serde message.
/// The associated path is a JSON Pointer relative to the wire document root.
///
/// # Examples
///
/// ```rust
/// use qubit_config::ConfigWireRepresentationErrorKind;
///
/// let reason = ConfigWireRepresentationErrorKind::InvalidType;
/// assert_eq!(format!("{reason:?}"), "InvalidType");
/// ```
#[non_exhaustive]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigWireRepresentationErrorKind {
    /// A required envelope or property field is absent.
    MissingField,
    /// A field has a JSON type incompatible with its wire contract.
    InvalidType,
    /// An envelope or property contains a field outside its accepted set.
    UnknownField,
    /// A value is well-typed JSON but violates the wire value contract.
    InvalidValue,
}
