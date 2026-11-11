// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Properties File Configuration Source
//!
//! Loads configuration from Java `.properties` format files.
//!
//! # Format
//!
//! The `.properties` format supports:
//! - `key=value` assignments
//! - `key: value` assignments (colon separator)
//! - `key value` assignments (whitespace separator)
//! - `# comment` and `! comment` lines
//! - Blank lines (ignored)
//! - Line continuation with an odd number of `\` characters at end of line
//! - Java properties escape sequences (`\uXXXX`, `\=`, `\:`, `\ `, etc.)
//!
//! # Escape Dialect
//!
//! Valid `\uXXXX` escapes decode one UTF-16 code unit, and a valid high/low
//! surrogate pair decodes one Unicode scalar. A malformed or incomplete
//! Unicode escape is preserved verbatim so configuration data is not silently
//! discarded. As in Java properties files, an unknown non-Unicode escape
//! removes its leading backslash.
//!
//! ```rust
//! use qubit_config::source::PropertiesConfigSource;
//!
//! let pairs = PropertiesConfigSource::parse_content(r"broken=\u12G4")?;
//! assert_eq!(
//!     pairs,
//!     vec![("broken".to_string(), r"\u12G4".to_string())],
//! );
//! # Ok::<(), qubit_config::ConfigError>(())
//! ```

use std::path::Path;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::properties_syntax::has_line_continuation;
use super::internal::properties_syntax::is_properties_whitespace;
use super::internal::properties_syntax::parse_key_value;
use super::internal::properties_syntax::properties_line_offset;
use super::internal::properties_syntax::properties_natural_lines;
use super::internal::properties_syntax::unescape_properties;
use super::internal::text_location::text_location;
use super::source_input::SourceInput;
use super::source_load_session::SourceLoadSession;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::SourceLocation;
use crate::config_path::ensure_config_key;

// Defines PropertiesConfigSource construction without exposing its stored
// fields.
#[path = "properties_config_source_builder.rs"]
mod properties_config_source_builder;
pub use properties_config_source_builder::PropertiesConfigSourceBuilder;

/// Configuration source that loads from Java `.properties` format files
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{PropertiesConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// let temp_dir = tempfile::tempdir().unwrap();
/// let path = temp_dir.path().join("config.properties");
/// std::fs::write(&path, "server.port=8080\n").unwrap();
/// let source = PropertiesConfigSource::from_file(path);
/// let config = source.load().unwrap();
/// let value = config.get::<String>("server.port").unwrap();
/// assert_eq!(value, "8080");
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone)]
pub struct PropertiesConfigSource {
    /// File or in-memory content to load.
    input: SourceInput,
    /// Resource limits applied while loading.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

impl PropertiesConfigSource {
    /// Creates a builder initialized with empty in-memory content.
    ///
    /// # Returns
    ///
    /// A builder ready to configure the source input and limits.
    #[inline]
    pub fn builder() -> PropertiesConfigSourceBuilder {
        PropertiesConfigSourceBuilder::new()
    }

    /// Creates a new `PropertiesConfigSource` from a file path.
    ///
    /// # Type Parameters
    ///
    /// * `P` - A path-like value that can be borrowed as a filesystem path.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.properties` file.
    ///
    /// # Returns
    ///
    /// A source configured to load the specified file.
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::builder().file(path).build()
    }

    /// Creates a properties source backed by in-memory text.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse when the source is loaded.
    ///
    /// # Returns
    ///
    /// A source configured to load the supplied text.
    #[inline]
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::builder().content(content).build()
    }

    /// Parses a `.properties` format string into ordered key-value pairs.
    ///
    /// # Parameters
    ///
    /// * `content` - The content of the `.properties` file
    ///
    /// # Returns
    ///
    /// Returns all parsed `(key, value)` pairs in input order. No partial
    /// result is returned if parsing, key validation, or a default source
    /// limit fails. Malformed or incomplete `\u` escapes are preserved
    /// verbatim; valid surrogate pairs decode to one Unicode scalar. A
    /// non-canonical key parse error includes its 1-based physical line; a
    /// continued assignment is attributed to its first physical line. This
    /// line number is diagnostic text and is distinct from `source_index()`.
    ///
    /// Input UTF-8 bytes are admitted before parsing. Each parsed assignment
    /// is checked against node, assignment, and semantic-depth limits.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] when an input,
    /// node, assignment, or depth limit is exceeded, or
    /// [`crate::ConfigError::SourceParseError`] for a non-canonical key.
    #[inline]
    pub fn parse_content(content: &str) -> ConfigResult<Vec<(String, String)>> {
        Self::parse_content_with_limits(content, SourceLimits::default())
    }

    /// Parses properties text under explicit source limits.
    ///
    /// Input UTF-8 bytes are admitted before parsing. Every parsed assignment
    /// is then checked against node, assignment, and semantic-depth limits.
    /// The returned pairs preserve input order; an error returns no partial
    /// result. Malformed or incomplete `\u` escapes are preserved verbatim.
    /// A non-canonical key parse error includes its 1-based physical line;
    /// continued assignments use their first physical line. This line number
    /// is diagnostic text and is distinct from `source_index()`.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse.
    /// * `limits` - Resource limits applied before and during parsing.
    ///
    /// # Returns
    ///
    /// Parsed key-value pairs in input order.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ConfigError::SourceLimitExceeded`] when an input,
    /// node, assignment, or depth limit is exceeded, or
    /// [`crate::ConfigError::SourceParseError`] for a non-canonical key.
    #[inline]
    pub fn parse_content_with_limits(content: &str, limits: SourceLimits) -> ConfigResult<Vec<(String, String)>> {
        Self::parse_content_with_source(content, limits, "properties:<memory>")
    }

    /// Parses properties text using an explicit source identifier for
    /// diagnostics.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse.
    /// * `limits` - Resource limits applied before and during parsing.
    /// * `source_id` - Identifier attached to parse and limit errors.
    ///
    /// # Returns
    ///
    /// Parsed key-value pairs in input order.
    ///
    /// # Errors
    ///
    /// Returns an input, node, assignment, depth, or key validation error.
    fn parse_content_with_source(
        content: &str,
        limits: SourceLimits,
        source_id: &str,
    ) -> ConfigResult<Vec<(String, String)>> {
        let mut session = SourceLoadSession::new(source_id, limits);
        session.consume_input_bytes(content.len())?;
        Self::parse_content_with_session(content, source_id, &mut session)
    }

    /// Parses properties text through an existing load session.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse.
    /// * `source_id` - Identifier attached to parse and limit errors.
    /// * `session` - Existing source-load budgets to charge while parsing.
    ///
    /// # Returns
    ///
    /// Parsed key-value pairs in input order.
    ///
    /// # Errors
    ///
    /// Returns an input, node, assignment, depth, or key validation error.
    fn parse_content_with_session(
        content: &str,
        source_id: &str,
        session: &mut SourceLoadSession<'_>,
    ) -> ConfigResult<Vec<(String, String)>> {
        let mut result = Vec::new();
        Self::parse_content_into(content, source_id, session, |key, value| {
            result.push((key, value));
            Ok(())
        })?;
        Ok(result)
    }

    /// Parses properties content and forwards each budget-admitted assignment
    /// to a receiver. The receiver is invoked only after key and budget checks.
    ///
    /// # Parameters
    ///
    /// * `content` - Properties text to parse.
    /// * `source_id` - Identifier used to attribute parse errors.
    /// * `session` - Active source-load session for resource accounting.
    /// * `receive` - Sink for each parsed key/value assignment.
    ///
    /// # Returns
    ///
    /// `Ok(())` after every assignment is parsed, budgeted, and accepted by the
    /// receiver.
    ///
    /// # Errors
    ///
    /// Returns the first parse, budget, or receiver error. Parse and budget
    /// errors discovered later retain precedence over a receiver error.
    fn parse_content_into(
        content: &str,
        source_id: &str,
        session: &mut SourceLoadSession<'_>,
        mut receive: impl FnMut(String, String) -> ConfigResult<()>,
    ) -> ConfigResult<()> {
        let mut lines = properties_natural_lines(content).enumerate();
        let mut receive_error = None;

        while let Some((line_index, line)) = lines.next() {
            let start_line = line_index + 1;
            let trimmed = line.trim_start_matches(is_properties_whitespace);

            // Skip blank lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                continue;
            }

            // Handle line continuation
            let mut full_line = trimmed.to_string();
            let mut continued = has_line_continuation(trimmed);
            while continued {
                full_line.pop(); // remove trailing backslash
                let Some((_, next)) = lines.next() else {
                    break;
                };
                let next = next.trim_start_matches(is_properties_whitespace);
                continued = has_line_continuation(next);
                full_line.push_str(next);
            }

            // Parse key/value pairs using Java properties separators.
            if let Some((key, value)) = parse_key_value(&full_line) {
                let key = unescape_properties(key);
                let value = unescape_properties(value);
                ensure_config_key(key.as_str()).map_err(|error| {
                    let error = ConfigError::source_parse_error_at(
                        source_id,
                        Some(key.clone()),
                        None,
                        format!("{error} at line {start_line}"),
                    );
                    let line = text_location(content, properties_line_offset(content, start_line))
                        .expect("physical line starts are valid UTF-8 boundaries")
                        .line();
                    error.with_source_location(SourceLocation::new(
                        std::num::NonZeroUsize::new(line).expect("physical line numbers are non-zero"),
                        None,
                    ))
                })?;
                session.check_depth(key.split('.').count())?;
                session.consume_nodes(1)?;
                session.consume_properties(1)?;
                if receive_error.is_none()
                    && let Err(error) = receive(key, value)
                {
                    // Preserve the old parse-first error order: keep checking
                    // later syntax and budgets, which take precedence.
                    receive_error = Some(error);
                }
            }
        }

        receive_error.map_or(Ok(()), Err)
    }
}

impl ConfigSource for PropertiesConfigSource {
    /// Returns the stable identifier used to report this source during loading.
    fn source_id(&self) -> String {
        self.label.clone().unwrap_or_else(|| self.input.label("properties"))
    }

    /// Returns the resource limits applied to one load of this source.
    #[inline]
    fn limits(&self) -> SourceLimits {
        self.limits
    }

    /// Loads source data into the executor-provided context.
    ///
    /// # Parameters
    ///
    /// * `context` - Context for charging resource use and writing the source
    ///   layer.
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        let mut config = Config::new();
        let session = context.session_mut();
        let source_id = self.source_id();
        let content = self.input.read_to_string("properties", session)?;
        Self::parse_content_into(&content, &source_id, session, |key, value| {
            config
                .set(&key, value)
                .map_err(|error| error.with_source_context(&source_id, Some(key), None))
        })?;
        context.replace_layer(config);
        Ok(())
    }
}
