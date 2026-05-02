/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

//! Ergonomic configuration key list argument adapters.

/// Provides borrowed access to a candidate configuration key list.
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
