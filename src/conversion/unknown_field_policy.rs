// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavior for fields that a structured target does not consume.
//!
//! [`UnknownFieldPolicy::Reject`] rejects fields Serde reports through
//! `deserialize_ignored_any`. This detects ordinary derived struct fields,
//! but is not complete schema validation for every `DeserializeOwned`
//! implementation. Open maps and flatten maps can consume arbitrary keys;
//! flatten structs, untagged enums, and custom `Deserialize` implementations
//! that buffer values can hide discarded fields from `serde_ignored`.
//!
//! When an application must reject misspelled keys, use
//! `#[serde(deny_unknown_fields)]` on a closed struct. For an untagged enum,
//! apply it to every struct payload variant. Serde's rejection is reported as
//! a sanitized deserialization error and is not guaranteed to produce the
//! [`crate::ConfigError::UnknownProperties`] path list. `deny_unknown_fields`
//! cannot be combined with `flatten`; replace a flatten struct with an ordinary
//! nested closed struct, or validate the key set against the application's
//! schema before deserializing.

/// Controls how structured reads handle unknown fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    /// Reject fields Serde exposes as ignored, with sorted root-relative paths.
    ///
    /// This is not complete schema validation for all target types. Flatten
    /// structs, untagged enums, and custom `Deserialize` implementations can
    /// buffer fields so that `serde_ignored` cannot observe them. Use Serde's
    /// `deny_unknown_fields` on closed struct targets when all unknown fields
    /// must be rejected; its errors are sanitized deserialization errors, not
    /// necessarily [`crate::ConfigError::UnknownProperties`].
    #[default]
    Reject,
    /// Ignore unconsumed fields after their complete input admission.
    Ignore,
}
