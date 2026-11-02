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
    /// Creates a builder with empty in-memory content.
    pub fn new() -> Self {
        Self {
            input: SourceInput::Content(String::new()),
            limits: SourceLimits::default(),
            label: None,
        }
    }

    /// Sets in-memory properties content.
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

    /// Sets a properties file path.
    pub fn file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input = SourceInput::File(path.as_ref().to_path_buf());
        self
    }

    /// Sets source resource limits.
    pub const fn limits(mut self, limits: SourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builds the configured properties source.
    pub fn build(self) -> PropertiesConfigSource {
        PropertiesConfigSource {
            input: self.input,
            limits: self.limits,
            label: self.label,
        }
    }
}

impl Default for PropertiesConfigSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
