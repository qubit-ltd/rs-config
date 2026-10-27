// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared file and in-memory input handling for text sources.

use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use super::source_load_session::SourceLoadSession;
use crate::ConfigError;
use crate::ConfigResult;

/// Input backing one built-in text configuration source.
#[derive(Clone)]
pub(crate) enum SourceInput {
    /// File-backed source.
    File(PathBuf),
    /// In-memory source.
    Content(String),
}

impl std::fmt::Debug for SourceInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(path) => formatter.debug_tuple("File").field(path).finish(),
            Self::Content(content) => formatter
                .debug_struct("Content")
                .field("bytes", &content.len())
                .field("value", &"<redacted>")
                .finish(),
        }
    }
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
    ///
    /// In-memory content is returned by reference after its byte budget is
    /// checked; file content is decoded into an owned string.
    pub(crate) fn read_to_string(
        &self,
        format: &str,
        session: &mut SourceLoadSession<'_>,
    ) -> ConfigResult<Cow<'_, str>> {
        let label = self.label(format);
        match self {
            Self::File(path) => {
                let file = File::open(path).map_err(|error| {
                    ConfigError::source_io_error(
                        path.display().to_string(),
                        std::io::Error::new(
                            error.kind(),
                            format!("Failed to open {format} file '{}': {error}", path.display()),
                        ),
                    )
                })?;
                let bytes = read_file_bytes(file, session, path, format)?;
                String::from_utf8(bytes).map(Cow::Owned).map_err(|error| {
                    ConfigError::source_io_error(
                        label.clone(),
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Failed to read {format} source '{label}': {error}"),
                        ),
                    )
                })
            }
            Self::Content(content) => {
                session.consume_input_bytes(content.len())?;
                Ok(Cow::Borrowed(content))
            }
        }
    }
}

/// Reads in chunks of at most 8 KiB, appending each chunk only after its byte
/// budget charge succeeds. A read may therefore consume one chunk beyond the
/// remaining budget before that charge is rejected; the result vector is not
/// extended with the rejected chunk.
fn read_file_bytes(
    file: File,
    session: &mut SourceLoadSession<'_>,
    path: &std::path::Path,
    format: &str,
) -> ConfigResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut reader = file;
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let count = reader.read(&mut chunk).map_err(|error| {
            ConfigError::source_io_error(
                path.display().to_string(),
                std::io::Error::new(
                    error.kind(),
                    format!("Failed to read {format} file '{}': {error}", path.display()),
                ),
            )
        })?;
        if count == 0 {
            break;
        }
        session.consume_input_bytes(count)?;
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(bytes)
}
