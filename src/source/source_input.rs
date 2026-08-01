// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared file and in-memory input handling for text sources.

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::{ConfigError, ConfigResult};

use super::{SourceLimits, source_budget::SourceBudget};

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
                        format!("Failed to open {format} file '{}': {error}", path.display()),
                    ))
                })?;
                read_file_bytes(file, limits.max_input_bytes(), path, format)?
            }
            Self::Content(content) => content.as_bytes().to_vec(),
        };
        budget.consume_input_bytes(bytes.len())?;
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
                format!("Failed to read {format} file '{}': {error}", path.display()),
            ))
        })?;
    } else {
        let mut reader = file.take(limit.saturating_add(1) as u64);
        reader.read_to_end(&mut bytes).map_err(|error| {
            ConfigError::IoError(std::io::Error::new(
                error.kind(),
                format!("Failed to read {format} file '{}': {error}", path.display()),
            ))
        })?;
    }
    Ok(bytes)
}
