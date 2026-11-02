// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::path::Path;

use super::EnvFileConfigSource;
use super::SourceInput;
use super::SourceLimits;

/// Builder for [`EnvFileConfigSource`].
#[must_use = "use the returned value"]
pub struct EnvFileConfigSourceBuilder {
    /// Selected file or in-memory content.
    input: SourceInput,
    /// Resource limits for the source load.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

impl EnvFileConfigSourceBuilder {
    /// Creates a builder using empty in-memory content and default source
    /// limits.
    ///
    /// The empty content is not parsed until the built source is loaded.
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
    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    /// Replaces the resource limits enforced while reading and parsing the
    /// source.
    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the source without reading or parsing its selected input.
    ///
    /// Input and limit failures remain deferred until the source is loaded
    /// through [`crate::source::ConfigSource`].
    pub fn build(self) -> EnvFileConfigSource {
        EnvFileConfigSource {
            input: self.input,
            limits: self.limits,
            label: self.label,
        }
    }
}

impl Default for EnvFileConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
