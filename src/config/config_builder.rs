// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use qubit_utils::Transient;

use super::Config;
use crate::options::ReadPolicy;

/// Builder for [`Config`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::Config;
/// use qubit_config::ReadPolicy;
///
/// let policy = ReadPolicy::builder().max_interpolation_depth(8).build();
/// let mut config = Config::builder()
///     .description("service settings")
///     .default_read_policy(policy)
///     .build();
/// config.set("host", "example.test")?;
/// config.set("endpoint", "https://${host}")?;
/// assert_eq!(config.get::<String>("endpoint")?, "https://${host}");
/// assert_eq!(config.get_interpolated::<String>("endpoint")?, "https://example.test");
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    /// Optional description copied into the built configuration.
    description: Option<String>,
    /// Read policy installed as the configuration's default.
    default_read_policy: ReadPolicy,
}

impl ConfigBuilder {
    /// Creates a builder initialized with an empty configuration.
    ///
    /// # Returns
    ///
    /// A builder with no description and the default read policy.
    #[inline]
    pub fn new() -> Self {
        Self {
            description: None,
            default_read_policy: ReadPolicy::default(),
        }
    }

    /// Sets the configuration description.
    ///
    /// # Parameters
    ///
    /// * `description` - Description to store in the built configuration.
    ///
    /// # Returns
    ///
    /// The builder with the supplied description.
    #[inline]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Sets the default runtime read policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - Policy to use for direct reads on the built configuration.
    ///
    /// # Returns
    ///
    /// The builder with the supplied default read policy.
    #[inline]
    pub fn default_read_policy(mut self, policy: ReadPolicy) -> Self {
        self.default_read_policy = policy;
        self
    }

    /// Builds the configured empty configuration.
    ///
    /// # Returns
    ///
    /// An empty configuration with the builder's description and read policy.
    #[inline]
    pub fn build(self) -> Config {
        Config {
            description: self.description,
            properties: BTreeMap::new(),
            default_read_policy: Transient::new(self.default_read_policy),
        }
    }
}

impl Default for ConfigBuilder {
    /// Creates a builder with no description and the default read policy.
    ///
    /// # Returns
    ///
    /// The same initial builder as [`ConfigBuilder::new`].
    fn default() -> Self {
        Self::new()
    }
}
