// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::path::Path;

use super::SourceInput;
use super::SourceLimits;
use super::TomlConfigSource;

/// Builder for [`TomlConfigSource`].
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{ConfigSource, TomlConfigSourceBuilder};
///
/// let source = TomlConfigSourceBuilder::new().content("server.port = 8080").build();
/// let config = source.load()?;
/// assert_eq!(config.get::<i64>("server.port")?, 8080);
/// # Ok::<(), qubit_config::ConfigError>(())
/// ```
#[must_use = "configure the TOML source and call build"]
pub struct TomlConfigSourceBuilder {
    /// Selected file or in-memory content.
    input: SourceInput,
    /// Resource limits for the source load.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

impl TomlConfigSourceBuilder {
    /// Creates a builder using empty in-memory content and default source
    /// limits.
    ///
    /// The empty content is not parsed until the built source is loaded.
    ///
    /// # Returns
    ///
    /// A builder initialized with empty content and default limits.
    #[inline]
    pub fn new() -> Self {
        Self {
            input: SourceInput::Content(String::new()),
            limits: SourceLimits::default(),
            label: None,
        }
    }

    /// Uses `content` as the source input.
    ///
    /// The owned content replaces any file or content selected earlier. It is
    /// parsed only when the built source is loaded, when format and resource
    /// limit errors are reported through [`crate::source::ConfigSource`].
    ///
    /// # Parameters
    ///
    /// * `content` - TOML text to use as the source input.
    ///
    /// # Returns
    ///
    /// The updated builder.
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
    /// * `label` - Optional diagnostic label; an empty label restores the
    ///   default.
    ///
    /// # Returns
    ///
    /// The updated builder.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        self.label = if label.is_empty() { None } else { Some(label) };
        self
    }

    /// Uses `path` as the source input.
    ///
    /// The path replaces any file or content selected earlier. The file is not
    /// opened until the built source is loaded, so I/O, format, and resource
    /// limit errors are reported by [`crate::source::ConfigSource`].
    ///
    /// # Type Parameters
    ///
    /// * `P` - Path-like type accepted by `AsRef<Path>`.
    ///
    /// # Parameters
    ///
    /// * `path` - TOML file path to use as the source input.
    ///
    /// # Returns
    ///
    /// The updated builder.
    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    /// Replaces the limits used for input admission, parser admission, and
    /// flattening.
    ///
    /// Input bytes, lexer tokens, and conservative parser nesting depth are
    /// admitted before the backend builds its value tree. Node, assignment,
    /// and semantic output-depth limits are checked while flattening that tree.
    /// Parser depth is distinct from semantic depth; parser limits do not
    /// guarantee a strict allocation or CPU ceiling.
    ///
    /// See [`SourceLimits`],
    /// [`SourceLimitsBuilder::max_parser_tokens`](crate::source::SourceLimitsBuilder::max_parser_tokens),
    /// and [`SourceLimitsBuilder::max_parser_nesting_depth`](crate::source::SourceLimitsBuilder::max_parser_nesting_depth).
    ///
    /// # Parameters
    ///
    /// * `limits` - Limits to apply during input admission, parsing, and
    ///   flattening.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline]
    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the source without reading or parsing its selected input.
    ///
    /// Input and limit failures remain deferred until the source is loaded
    /// through [`crate::source::ConfigSource`].
    ///
    /// # Returns
    ///
    /// The configured TOML source, without reading or parsing its input.
    #[inline]
    pub fn build(self) -> TomlConfigSource {
        TomlConfigSource {
            input: self.input,
            limits: self.limits,
            label: self.label,
        }
    }
}

impl Default for TomlConfigSourceBuilder {
    /// Creates the default value for this type.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
