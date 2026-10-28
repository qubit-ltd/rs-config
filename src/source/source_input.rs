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
    mut reader: impl Read,
    session: &mut SourceLoadSession<'_>,
    path: &std::path::Path,
    format: &str,
) -> ConfigResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(ConfigError::source_io_error(
                    path.display().to_string(),
                    std::io::Error::new(
                        error.kind(),
                        format!("Failed to read {format} file '{}': {error}", path.display()),
                    ),
                ));
            }
        };
        if count == 0 {
            break;
        }
        session.consume_input_bytes(count)?;
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::io::Error;
    use std::io::ErrorKind;
    use std::io::Read;
    use std::path::Path;

    use super::read_file_bytes;
    use crate::ConfigError;
    use crate::SourceLimitKind;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    struct InterruptedOnce {
        pending: bool,
        inner: Cursor<Vec<u8>>,
    }

    impl Read for InterruptedOnce {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            if self.pending {
                self.pending = false;
                return Err(Error::from(ErrorKind::Interrupted));
            }
            self.inner.read(output)
        }
    }

    struct ScriptedReader {
        steps: VecDeque<Result<Vec<u8>, ErrorKind>>,
    }

    impl Read for ScriptedReader {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            match self.steps.pop_front() {
                Some(Ok(bytes)) => {
                    output[..bytes.len()].copy_from_slice(&bytes);
                    Ok(bytes.len())
                }
                Some(Err(kind)) => Err(Error::from(kind)),
                None => Ok(0),
            }
        }
    }

    #[test]
    fn retries_interrupted_reads_without_double_charging() {
        let reader = InterruptedOnce {
            pending: true,
            inner: Cursor::new(b"a=1\n".to_vec()),
        };
        let limits = SourceLimits::builder().max_input_bytes(4).build();
        let mut session = SourceLoadSession::new("input-test", limits);
        let bytes = read_file_bytes(reader, &mut session, Path::new("input-test"), "properties")
            .expect("an interrupted read should be retried without charging input");

        assert_eq!(bytes, b"a=1\n");
    }

    #[test]
    fn retries_multiple_interruptions_after_partial_reads() {
        let reader = ScriptedReader {
            steps: VecDeque::from([
                Err(ErrorKind::Interrupted),
                Ok(b"a=".to_vec()),
                Err(ErrorKind::Interrupted),
                Ok(b"1\n".to_vec()),
            ]),
        };
        let limits = SourceLimits::builder().max_input_bytes(4).build();
        let mut session = SourceLoadSession::new("input-test", limits);
        let bytes = read_file_bytes(reader, &mut session, Path::new("input-test"), "properties")
            .expect("interruptions after partial reads should not charge bytes");

        assert_eq!(bytes, b"a=1\n");
    }

    #[test]
    fn input_over_limit_still_reports_the_rejected_chunk() {
        let limits = SourceLimits::builder().max_input_bytes(3).build();
        let mut session = SourceLoadSession::new("input-test", limits);
        let error = read_file_bytes(
            Cursor::new(b"a=1\n".to_vec()),
            &mut session,
            Path::new("input-test"),
            "properties",
        )
        .expect_err("the full read exceeds the byte allowance");

        assert!(matches!(
            error,
            ConfigError::SourceLimitExceeded {
                source_id,
                kind: SourceLimitKind::InputBytes,
                limit: 3,
                observed_at_least: 4,
                ..
            } if source_id == "input-test"
        ));
    }

    #[test]
    fn permanent_reader_errors_keep_the_original_kind_and_source() {
        struct BrokenReader;

        impl Read for BrokenReader {
            fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
                Err(Error::other("reader failure"))
            }
        }

        let mut session = SourceLoadSession::new("input-test", SourceLimits::default());
        let error = read_file_bytes(BrokenReader, &mut session, Path::new("input-test"), "properties")
            .expect_err("permanent I/O errors are returned");

        assert!(matches!(error, ConfigError::SourceIoError { .. }));
        assert_eq!(error.source_id(), Some("input-test"));
        let io_error = std::error::Error::source(&error)
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .expect("source I/O error should retain its underlying io::Error");
        assert_eq!(io_error.kind(), ErrorKind::Other);
    }
}
