// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Configuration traversal and subtree operations.

use std::ops::Bound;

use super::Config;
use crate::ConfigResult;
use crate::Property;
use crate::config_path::ensure_config_path;

impl Config {
    // ========================================================================
    // Prefix Traversal and Sub-tree Extraction (v0.4.0)
    // ========================================================================

    /// Iterates over all configuration entries as `(key, &Property)` pairs.
    ///
    /// # Returns
    ///
    /// An iterator yielding `(&str, &Property)` tuples.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("host", "localhost").unwrap();
    /// config.set("port", 8080).unwrap();
    ///
    /// for (key, prop) in config.iter() {
    ///     println!("{} = {:?}", key, prop);
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Property)> {
        self.properties.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterates over all configuration entries whose key starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to filter by (e.g., `"http."`)
    ///
    /// # Returns
    ///
    /// An iterator of `(&str, &Property)` whose keys start with `prefix`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost").unwrap();
    /// config.set("http.port", 8080).unwrap();
    /// config.set("db.host", "dbhost").unwrap();
    ///
    /// let http_entries: Vec<_> = config.iter_prefix("http.").collect();
    /// assert_eq!(http_entries.len(), 2);
    /// ```
    #[inline]
    pub fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Property)> {
        self.properties
            .range::<str, _>((Bound::Included(prefix), Bound::Unbounded))
            .take_while(move |(key, _)| key.starts_with(prefix))
            .map(|(key, value)| (key.as_str(), value))
    }

    /// Returns `true` if any configuration key starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - The key prefix to check
    ///
    /// # Returns
    ///
    /// `true` if at least one key starts with `prefix`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// config.set("http.host", "localhost").unwrap();
    ///
    /// assert!(config.contains_key_prefix("http."));
    /// assert!(!config.contains_key_prefix("db."));
    /// ```
    #[inline]
    pub fn contains_key_prefix(&self, prefix: &str) -> bool {
        self.iter_prefix(prefix).next().is_some()
    }

    /// Returns whether a dotted configuration section has descendants.
    ///
    /// An exact scalar at `path` and sibling names such as `proxy2` do not
    /// make the `proxy` section present.
    ///
    /// # Parameters
    ///
    /// * `path` - Canonical dotted section path. An empty path represents the
    ///   root.
    ///
    /// # Returns
    ///
    /// `true` when at least one descendant key belongs to the section.
    #[inline]
    pub fn contains_section(&self, path: &str) -> ConfigResult<bool> {
        ensure_config_path(path)?;
        if path.is_empty() {
            return Ok(!self.properties.is_empty());
        }
        let child_prefix = format!("{path}.");
        Ok(self.contains_key_prefix(&child_prefix))
    }
}
