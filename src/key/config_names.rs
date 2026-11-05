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
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by `operation`.
    ///
    /// # Parameters
    ///
    /// * `operation` - Closure invoked with the candidate names in order.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R;
}

impl ConfigNames for &[&str] {
    /// Runs the operation with this borrowed name slice.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate names in lookup order.
    /// * `operation` - Closure invoked with the candidate names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self)
    }
}

impl<const N: usize> ConfigNames for [&str; N] {
    /// Runs the operation with this owned fixed-size name array.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate names in lookup order.
    /// * `operation` - Closure invoked with the candidate names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(&self)
    }
}

impl<const N: usize> ConfigNames for &[&str; N] {
    /// Runs the operation with this borrowed fixed-size name array.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate names in lookup order.
    /// * `operation` - Closure invoked with the candidate names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for Vec<&str> {
    /// Runs the operation with this owned string-slice vector.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate names in lookup order.
    /// * `operation` - Closure invoked with the candidate names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for &Vec<&str> {
    /// Runs the operation with this borrowed string-slice vector.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate names in lookup order.
    /// * `operation` - Closure invoked with the candidate names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        operation(self.as_slice())
    }
}

impl ConfigNames for &[String] {
    /// Converts the strings to borrowed names and runs the operation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate strings in lookup order.
    /// * `operation` - Closure invoked with the borrowed names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl<const N: usize> ConfigNames for [String; N] {
    /// Converts the strings to borrowed names and runs the operation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate strings in lookup order.
    /// * `operation` - Closure invoked with the borrowed names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl<const N: usize> ConfigNames for &[String; N] {
    /// Converts the strings to borrowed names and runs the operation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate strings in lookup order.
    /// * `operation` - Closure invoked with the borrowed names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl ConfigNames for Vec<String> {
    /// Converts the strings to borrowed names and runs the operation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate strings in lookup order.
    /// * `operation` - Closure invoked with the borrowed names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}

impl ConfigNames for &Vec<String> {
    /// Converts the strings to borrowed names and runs the operation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Result type returned by the operation.
    ///
    /// # Parameters
    ///
    /// * `self` - Candidate strings in lookup order.
    /// * `operation` - Closure invoked with the borrowed names.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    fn with_config_names<R>(self, operation: impl FnOnce(&[&str]) -> R) -> R {
        let names: Vec<&str> = self.iter().map(String::as_str).collect();
        operation(names.as_slice())
    }
}
