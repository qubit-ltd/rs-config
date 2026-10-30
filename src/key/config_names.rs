// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ergonomic configuration key list argument adapters.

/// Provides borrowed access to a candidate configuration key list.
///
/// Candidate order is preserved by this adapter. Configuration lookup APIs
/// check names in that order and return the first present value.
///
/// # Examples
///
/// ```rust
/// use qubit_config::{Config, ConfigNames};
///
/// let candidates = ["preferred", "fallback"];
/// candidates.with_config_names(|names| assert_eq!(names[0], "preferred"));
///
/// let mut config = Config::new();
/// config.set("fallback", 20)?;
/// config.set("preferred", 10)?;
/// assert_eq!(config.get_any::<i32>(candidates)?, 10);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
pub trait ConfigNames {
    /// Invokes `operation` with this argument as a string slice list.
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R;
}

impl ConfigNames for &[&str] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self)
    }
}

impl<const N: usize> ConfigNames for [&str; N] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(&self)
    }
}

impl<const N: usize> ConfigNames for &[&str; N] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for Vec<&str> {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for &Vec<&str> {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for &[String] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl<const N: usize> ConfigNames for [String; N] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl<const N: usize> ConfigNames for &[String; N] {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl ConfigNames for Vec<String> {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl ConfigNames for &Vec<String> {
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}
