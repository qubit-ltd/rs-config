/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Base Implementation of Configurable Objects
//!
//! Provides a base structure that implements the `Configurable` trait.
//!
//! # Author
//!
//! Haixing Hu

use super::{Config, Configurable};

/// Base implementation of configurable objects
///
/// This is a base structure that implements the `Configurable` trait and can be
/// used as a base for other structures that need configuration.
///
/// # Features
///
/// - Automatically implements the `Configurable` trait
/// - Provides configuration change callback mechanism
/// - Can be inherited and extended
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, Configurable, Configured};
///
/// let mut configured = Configured::new();
/// configured.config_mut().set("port", 8080).unwrap();
/// let port: i32 = configured.config().get("port").unwrap();
/// assert_eq!(port, 8080);
/// ```
///
/// ```rust
/// // Or compose it into other structures
/// use qubit_config::{Config, Configurable, Configured};
/// struct Server {
///     configured: Configured,
///     // Other fields...
/// }
///
/// impl Server {
///     fn new() -> Self {
///         Self {
///             configured: Configured::new(),
///         }
///     }
///
///     fn config(&self) -> &Config {
///         self.configured.config()
///     }
///
///     fn config_mut(&mut self) -> &mut Config {
///         self.configured.config_mut()
///     }
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Clone, PartialEq)]
pub struct Configured {
    /// Configuration object
    config: Config,
}

impl Configured {
    /// Creates a new configurable object
    ///
    /// # Returns
    ///
    /// Returns a new configurable object instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::{Configurable, Configured};
    ///
    /// let configured = Configured::new();
    /// assert!(configured.config().is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            config: Config::new(),
        }
    }

    /// Creates a configurable object with the specified configuration
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration object
    ///
    /// # Returns
    ///
    /// Returns a new configurable object instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::{Config, Configurable, Configured};
    ///
    /// let mut configured = Configured::with_config(Config::new());
    /// assert!(configured.config().is_empty());
    /// ```
    #[inline]
    pub fn with_config(config: Config) -> Self {
        Self { config }
    }
}

impl Configurable for Configured {
    #[inline]
    fn config(&self) -> &Config {
        &self.config
    }

    #[inline]
    fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    fn set_config(&mut self, config: Config) {
        self.config = config;
        self.on_config_changed();
    }

    #[inline]
    fn on_config_changed(&mut self) {
        // Default implementation is empty, subclasses can override
    }
}

impl Default for Configured {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
