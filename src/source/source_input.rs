// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Shared file and in-memory input handling for text sources.

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::{
    ConfigError,
    ConfigResult,
};

use super::{
    SourceLimits,
    source_budget::SourceBudget,
};

/// Input backing one built-in text configuration source.
#[derive(Debug, Clone)]
pub(crate) enum SourceInput {
    /// File-backed source.
    File(PathBuf),
    /// In-memory source.
    Content(String),
}

impl SourceInput {
    /// Returns a stable, content-free source label.
    pub(crate) fn label(&self, format: &str) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::Content(_) => format!("{format}:<memory>"),
        }
    }

    /// Reads and validates one UTF-8 source document.
    pub(crate) fn read_to_string(
        &self,
        format: &str,
        limits: SourceLimits,
    ) -> ConfigResult<String> {
        let label = self.label(format);
        let mut budget = SourceBudget::new(&label, limits);
        let bytes = match self {
            Self::File(path) => {
                let file = File::open(path).map_err(|error| {
                    ConfigError::IoError(std::io::Error::new(
                        error.kind(),
                        format!(
                            "Failed to open {format} file '{}': {error}",
                            path.display()
                        ),
                    ))
                })?;
                read_file_bytes(file, limits.max_input_bytes(), path, format)?
            }
            Self::Content(content) => {
                // Check the declared in-memory length before allocating a
                // second owned buffer for the source bytes.
                budget.consume_input_bytes(content.len())?;
                content.as_bytes().to_vec()
            }
        };
        if matches!(self, Self::File(_)) {
            budget.consume_input_bytes(bytes.len())?;
        }
        String::from_utf8(bytes).map_err(|error| {
            ConfigError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read {format} source '{label}': {error}"),
            ))
        })
    }
}

/// Reads at most one byte beyond a finite file limit.
fn read_file_bytes(
    file: File,
    limit: usize,
    path: &std::path::Path,
    format: &str,
) -> ConfigResult<Vec<u8>> {
    let mut bytes = Vec::new();
    if limit == usize::MAX {
        let mut reader = file;
        reader.read_to_end(&mut bytes).map_err(|error| {
            ConfigError::IoError(std::io::Error::new(
                error.kind(),
                format!(
                    "Failed to read {format} file '{}': {error}",
                    path.display()
                ),
            ))
        })?;
    } else {
        let mut reader = file.take(limit.saturating_add(1) as u64);
        reader.read_to_end(&mut bytes).map_err(|error| {
            ConfigError::IoError(std::io::Error::new(
                error.kind(),
                format!(
                    "Failed to read {format} file '{}': {error}",
                    path.display()
                ),
            ))
        })?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::SourceInput;
    use crate::source::SourceLimits;

    #[test]
    fn in_memory_input_is_rejected_before_copying_when_over_limit() {
        let input = SourceInput::Content("abcd".to_owned());
        let limits = SourceLimits::default().with_max_input_bytes(3);

        let error = input
            .read_to_string("properties", limits)
            .expect_err("oversized in-memory input must be rejected");
        assert!(matches!(
            error,
            crate::ConfigError::SourceLimitExceeded {
                kind: crate::SourceLimitKind::InputBytes,
                limit: 3,
                observed_at_least: 4,
                ..
            }
        ));
    }
}
