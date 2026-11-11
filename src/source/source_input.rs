// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared file and in-memory input handling for text sources.

use std::borrow::Cow;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::path::PathBuf;

use super::source_load_session::SourceLoadSession;
use crate::ConfigError;
use crate::ConfigResult;

/// Input backing one built-in text configuration source.
#[must_use = "store the selected input in a source builder"]
#[derive(Clone)]
pub(crate) enum SourceInput {
    /// Reads and decodes a filesystem-backed source as UTF-8 text.
    ///
    /// The path is opened when the source is loaded, and the resulting bytes
    /// are charged against the active input budget.
    File(
        /// Filesystem location read when the source is loaded.
        PathBuf,
    ),
    /// Uses owned source text without filesystem I/O.
    ///
    /// Its UTF-8 byte length is charged against the active input budget before
    /// the text is returned.
    Content(
        /// Owned source text supplied by the caller.
        String,
    ),
}

impl Debug for SourceInput {
    /// Formats the input kind without revealing in-memory source contents.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination buffer for the redacted debug structure.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the redacted input representation is written successfully.
    ///
    /// # Errors
    ///
    /// Returns the formatting error reported by the destination formatter.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
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
    /// Returns a stable source label without exposing in-memory content.
    ///
    /// File-backed input uses its displayed path; in-memory input uses the
    /// supplied format name followed by `:<memory>`.
    ///
    /// # Parameters
    ///
    /// * `format` - Format name used to identify in-memory input.
    ///
    /// # Returns
    ///
    /// A content-free source identifier suitable for diagnostics.
    #[must_use]
    pub(crate) fn label(&self, format: &str) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::Content(_) => format!("{format}:<memory>"),
        }
    }

    /// Reads and validates one UTF-8 source document.
    ///
    /// In-memory content is returned by reference after its byte budget is
    /// checked; file content is read, charged, and decoded into an owned
    /// string. Invalid UTF-8 and filesystem failures are reported as source
    /// I/O errors.
    ///
    /// # Parameters
    ///
    /// * `format` - Format name included in invalid UTF-8 diagnostics.
    /// * `session` - Source-load budgets charged for the input bytes.
    ///
    /// # Returns
    ///
    /// A borrowed string for in-memory content or owned UTF-8 text for files.
    ///
    /// # Errors
    ///
    /// Returns a source limit error when input bytes exceed the budget, or a
    /// source I/O error when opening, reading, or decoding the file fails.
    pub(crate) fn read_to_string(
        &self,
        format: &str,
        session: &mut SourceLoadSession<'_>,
    ) -> ConfigResult<Cow<'_, str>> {
        let label = session.source_id().to_owned();
        match self {
            Self::File(path) => {
                let file = File::open(path).map_err(|error| ConfigError::source_io_error(label.clone(), error))?;
                let bytes = read_file_bytes(file, session)?;
                String::from_utf8(bytes).map(Cow::Owned).map_err(|error| {
                    ConfigError::source_io_error(
                        label.clone(),
                        Error::new(
                            ErrorKind::InvalidData,
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
///
/// # Parameters
///
/// * `reader` - Byte reader for the source file.
/// * `session` - Source-load budget charged before each chunk is appended.
///
/// # Returns
///
/// All bytes read before EOF when every chunk is admitted.
///
/// # Errors
///
/// Returns a source limit error for a rejected chunk or a source I/O error
/// when a non-interrupted read fails.
fn read_file_bytes(mut reader: impl Read, session: &mut SourceLoadSession<'_>) -> ConfigResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(count) => count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(ConfigError::source_io_error(session.source_id().to_owned(), error));
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
    use std::error::Error as StdError;
    use std::fmt::Display;
    use std::fmt::Formatter;
    use std::fmt::Result as FmtResult;
    use std::io::Cursor;
    use std::io::Error;
    use std::io::ErrorKind;
    use std::io::Read;
    use std::io::Result as IoResult;

    use super::read_file_bytes;
    use crate::ConfigError;
    use crate::SourceLimitKind;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    struct FailingReader(Option<Error>);

    impl Read for FailingReader {
        fn read(&mut self, _output: &mut [u8]) -> IoResult<usize> {
            Err(self.0.take().expect("one read is expected"))
        }
    }

    #[derive(Debug)]
    struct InnerFailure(Error);

    impl Display for InnerFailure {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
            formatter.write_str("injected reader failure")
        }
    }

    impl StdError for InnerFailure {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            Some(&self.0)
        }
    }

    struct InterruptedOnce {
        pending: bool,
        inner: Cursor<Vec<u8>>,
    }

    impl Read for InterruptedOnce {
        fn read(&mut self, output: &mut [u8]) -> IoResult<usize> {
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
        fn read(&mut self, output: &mut [u8]) -> IoResult<usize> {
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
    fn test_retries_interrupted_reads_without_double_charging() {
        let reader = InterruptedOnce {
            pending: true,
            inner: Cursor::new(b"a=1\n".to_vec()),
        };
        let limits = SourceLimits::builder().max_input_bytes(4).build();
        let mut session = SourceLoadSession::new("input-test", limits);
        let bytes = read_file_bytes(reader, &mut session)
            .expect("an interrupted read should be retried without charging input");

        assert_eq!(bytes, b"a=1\n");
    }

    #[test]
    fn test_retries_multiple_interruptions_after_partial_reads() {
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
        let bytes =
            read_file_bytes(reader, &mut session).expect("interruptions after partial reads should not charge bytes");

        assert_eq!(bytes, b"a=1\n");
    }

    #[test]
    fn test_input_over_limit_still_reports_the_rejected_chunk() {
        let limits = SourceLimits::builder().max_input_bytes(3).build();
        let mut session = SourceLoadSession::new("input-test", limits);
        let error = read_file_bytes(Cursor::new(b"a=1\n".to_vec()), &mut session)
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
    fn test_permanent_reader_os_errors_keep_the_original_os_code() {
        let mut session = SourceLoadSession::new("input-test", SourceLimits::default());
        let error = read_file_bytes(FailingReader(Some(Error::from_raw_os_error(12345))), &mut session)
            .expect_err("permanent I/O errors are returned");

        assert_eq!(error.source_id(), Some("input-test"));
        let io_error = StdError::source(&error)
            .and_then(|source| source.downcast_ref::<Error>())
            .expect("source I/O error should retain its underlying io::Error");
        assert_eq!(io_error.raw_os_error(), Some(12345));
    }

    #[test]
    fn test_permanent_reader_errors_keep_custom_error_and_source_chain() {
        let mut session = SourceLoadSession::new("input-test", SourceLimits::default());
        let error = read_file_bytes(
            FailingReader(Some(Error::other(InnerFailure(Error::from_raw_os_error(12345))))),
            &mut session,
        )
        .expect_err("permanent I/O errors are returned");

        assert_eq!(error.source_id(), Some("input-test"));
        let io_error = StdError::source(&error)
            .and_then(|source| source.downcast_ref::<Error>())
            .expect("source I/O error should retain its underlying io::Error");
        let inner = io_error
            .get_ref()
            .and_then(|source| source.downcast_ref::<InnerFailure>())
            .expect("the custom error stored in io::Error should be preserved");
        assert_eq!(inner.0.raw_os_error(), Some(12345));
        assert_eq!(
            StdError::source(inner)
                .and_then(|source| source.downcast_ref::<Error>())
                .and_then(Error::raw_os_error),
            Some(12345),
        );
    }
}
