// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::EnvConfigOptionsBuilder;

/// Options controlling environment-variable key selection and normalization.
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{EnvConfigOptions, EnvConfigSource};
///
/// let options = EnvConfigOptions::builder()
///     .prefix("APP_")
///     .strip_prefix()
///     .double_underscores_to_dots()
///     .lowercase_keys()
///     .build();
/// let source = EnvConfigSource::from_options(options);
/// # let _ = source;
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvConfigOptions {
    /// Optional prefix used to select environment variables.
    pub(super) prefix: Option<String>,
    /// Whether the configured prefix is removed from loaded keys.
    pub(super) strip_prefix: bool,
    /// Whether double underscores are converted to dots.
    pub(super) double_underscores_to_dots: bool,
    /// Whether loaded keys are lowercased.
    pub(super) lowercase_keys: bool,
}

impl EnvConfigOptions {
    /// Creates a builder initialized with no prefix filter or transformation.
    #[inline]
    pub const fn builder() -> EnvConfigOptionsBuilder {
        EnvConfigOptionsBuilder::new()
    }
}

impl Default for EnvConfigOptions {
    fn default() -> Self {
        Self::builder().build()
    }
}
