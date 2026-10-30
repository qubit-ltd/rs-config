// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::EnvConfigOptions;

/// Builder for [`EnvConfigOptions`].
#[must_use = "use the returned value"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvConfigOptionsBuilder {
    /// Optional environment-variable prefix filter.
    prefix: Option<String>,
    /// Whether to remove the prefix from loaded keys.
    strip_prefix: bool,
    /// Whether to map double underscores to dots.
    double_underscores_to_dots: bool,
    /// Whether to lowercase loaded keys.
    lowercase_keys: bool,
}

impl EnvConfigOptionsBuilder {
    /// Creates a builder with no prefix filter or transformation.
    #[inline]
    pub const fn new() -> Self {
        Self {
            prefix: None,
            strip_prefix: false,
            double_underscores_to_dots: false,
            lowercase_keys: false,
        }
    }

    /// Restricts loading to variables whose names start with `prefix`.
    #[inline]
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Removes the configured prefix from loaded keys.
    #[inline]
    pub const fn strip_prefix(mut self) -> Self {
        self.strip_prefix = true;
        self
    }

    /// Converts double underscores in loaded keys to dots.
    #[inline]
    pub const fn double_underscores_to_dots(mut self) -> Self {
        self.double_underscores_to_dots = true;
        self
    }

    /// Lowercases loaded keys.
    #[inline]
    pub const fn lowercase_keys(mut self) -> Self {
        self.lowercase_keys = true;
        self
    }

    /// Builds the configured environment options.
    #[inline]
    pub fn build(self) -> EnvConfigOptions {
        EnvConfigOptions {
            prefix: self.prefix,
            strip_prefix: self.strip_prefix,
            double_underscores_to_dots: self.double_underscores_to_dots,
            lowercase_keys: self.lowercase_keys,
        }
    }
}

impl Default for EnvConfigOptionsBuilder {
    /// Creates environment options with no prefix filter or key transformation.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
