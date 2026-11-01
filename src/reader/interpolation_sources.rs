// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use serde::Deserialize;
use serde::Serialize;

/// Sources consulted while resolving an interpolated variable.
///
/// The default is [`ConfigOnly`](Self::ConfigOnly). Environment fallback is
/// consulted only when an interpolation uses
/// [`ConfigThenEnv`](Self::ConfigThenEnv).
///
/// # Examples
///
/// ```rust
/// use qubit_config::options::InterpolationSources;
/// use qubit_config::ReadPolicy;
///
/// let policy = ReadPolicy::builder()
///     .interpolation_sources(InterpolationSources::ConfigOnly)
///     .max_interpolation_expansions(12)
///     .build();
///
/// assert_eq!(policy.interpolation_sources(), InterpolationSources::ConfigOnly);
/// assert_eq!(policy.max_interpolation_expansions(), 12);
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolationSources {
    /// Resolve variables from configuration data only.
    ConfigOnly,
    /// Resolve from configuration data, then fall back to the process
    /// environment.
    ConfigThenEnv,
}

impl Default for InterpolationSources {
    /// Uses configuration data as the only interpolation source.
    #[inline]
    fn default() -> Self {
        Self::ConfigOnly
    }
}
