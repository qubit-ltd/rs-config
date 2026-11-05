// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::path::Path;

use super::PropertiesConfigSource;
use super::SourceInput;
use super::SourceLimits;

/// Builder for [`PropertiesConfigSource`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{ConfigSource, PropertiesConfigSource};
///
/// let source = PropertiesConfigSource::builder()
///     .content("server.port=8080")
///     .build();
/// let config = source.load()?;
/// assert_eq!(config.get::<String>("server.port")?, "8080");
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[must_use = "use the returned value"]
pub struct PropertiesConfigSourceBuilder {
    /// Selected file or in-memory content.
    input: SourceInput,
    /// Resource limits for the source load.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

impl PropertiesConfigSourceBuilder {
    /// Creates a builder with empty in-memory content, default limits, and no
    /// custom label.
    ///
    /// # Returns
    ///
    /// A builder ready to select content or a file and customize its limits.
    #[inline]
    pub fn new() -> Self {
        Self {
            input: SourceInput::Content(String::new()),
            limits: SourceLimits::default(),
            label: None,
        }
    }

    /// Selects in-memory properties text, replacing any previously selected
    /// file.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse when the source is loaded.
    ///
    /// # Returns
    ///
    /// The builder with the in-memory content selected.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.input = SourceInput::Content(content.into());
        self
    }

    /// Sets a content-free diagnostic label for this source.
    ///
    /// A non-empty label is used verbatim as the source identifier in errors
    /// and budget diagnostics, including for file-backed input. An empty
    /// label restores the default identifier. The last call wins, and the
    /// label is preserved when the input is changed. Labels need not be
    /// unique. Do not include secrets in labels.
    ///
    /// # Parameters
    ///
    /// * `label` - The diagnostic source label; an empty string clears it.
    ///
    /// # Returns
    ///
    /// The builder with the diagnostic label updated.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        self.label = if label.is_empty() { None } else { Some(label) };
        self
    }

    /// Selects a properties file, replacing any previously selected content.
    ///
    /// # Type Parameters
    ///
    /// * `P` - A path-like value that can be borrowed as a filesystem path.
    ///
    /// # Parameters
    ///
    /// * `path` - The properties file to load.
    ///
    /// # Returns
    ///
    /// The builder configured to read the file.
    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    /// Applies resource limits to input admission and parsed properties.
    ///
    /// # Parameters
    ///
    /// * `limits` - The source limits used during loading and parsing.
    ///
    /// # Returns
    ///
    /// The builder configured with these limits.
    #[inline]
    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Consumes the builder and creates the configured properties source.
    ///
    /// # Returns
    ///
    /// A source using the selected input, limits, and diagnostic label.
    #[inline]
    pub fn build(self) -> PropertiesConfigSource {
        PropertiesConfigSource {
            input: self.input,
            limits: self.limits,
            label: self.label,
        }
    }
}

impl Default for PropertiesConfigSourceBuilder {
    /// Creates a builder with the same defaults as [`Self::new`].
    ///
    /// # Returns
    ///
    /// A builder with empty content, default limits, and no custom label.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
