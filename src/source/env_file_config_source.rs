// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # `.env` File Configuration Source
//!
//! Loads configuration from `.env` format files (as used by dotenv tools).
//!
//! # Format
//!
//! The `.env` format supports:
//! - `KEY=VALUE` assignments
//! - `# comment` lines
//! - Quoted values: `KEY="value with spaces"` or `KEY='value'`
//! - Export prefix: `export KEY=VALUE` (the `export` keyword is ignored)

use std::path::Path;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::dotenv_line_state::DotenvLineState;
use super::source_input::SourceInput;
use crate::Config;
use crate::ConfigError;
use crate::ConfigKey;
use crate::ConfigResult;

// Defines EnvFileConfigSource construction without exposing its stored fields.
#[path = "env_file_config_source_builder.rs"]
mod env_file_config_source_builder;
pub use env_file_config_source_builder::EnvFileConfigSourceBuilder;

/// Configuration source that loads from `.env` format files
///
/// # Examples
///
/// ```rust
/// use qubit_config::source::{EnvFileConfigSource, ConfigSource};
/// use qubit_config::Config;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let temp_dir = tempfile::tempdir()?;
/// let path = temp_dir.path().join(".env");
/// std::fs::write(&path, "PORT=8080\n")?;
/// let source = EnvFileConfigSource::from_file(path);
/// let config = source.load()?;
/// let port = config.get::<String>("PORT")?;
/// assert_eq!(port, "8080");
/// # Ok(())
/// # }
/// ```
#[must_use = "use the returned value"]
#[derive(Debug, Clone)]
pub struct EnvFileConfigSource {
    /// File or in-memory content to load.
    input: SourceInput,
    /// Resource limits applied while loading.
    limits: SourceLimits,
    /// Optional diagnostic label for this source.
    label: Option<String>,
}

/// Maps a dotenv parser error without exposing the offending assignment.
///
/// The position returned for a line parse error is parser-local. Since input
/// may be pre-escaped before parsing, it does not identify a position in the
/// original file; its unit is not guaranteed to be a file column or byte
/// offset.
///
/// # Parameters
///
/// * `label` - Label of the dotenv source being loaded.
/// * `error` - Parser or I/O error returned by dotenvy.
///
/// # Returns
///
/// A source-aware I/O or value-redacted parse [`ConfigError`].
fn map_dotenv_error(label: &str, error: dotenvy::Error) -> ConfigError {
    match error {
        dotenvy::Error::Io(source) => ConfigError::source_io_error(
            label,
            std::io::Error::new(source.kind(), format!("Failed to read .env source '{label}': {source}")),
        ),
        dotenvy::Error::LineParse(_line, parser_offset) => ConfigError::source_parse_error(
            label,
            format!(
                "Failed to parse .env file '{label}' at parser-local offset {parser_offset} (not a file line or column): invalid assignment: <redacted>",
            ),
        ),
        _error => ConfigError::source_parse_error(
            label,
            format!(
                "Failed to parse .env file '{}': invalid dotenv input: <redacted>",
                label
            ),
        ),
    }
}

/// Removes recognized inline comment bodies while preserving line endings.
///
/// Both the local substitution scanner and dotenvy's value parser need the
/// comment boundary. Removing comment quotes and escapes before either pass
/// prevents them from changing scanner state. Parser-local offsets afterward
/// refer to normalized input and are not original-file coordinates.
///
/// # Parameters
///
/// * `content` - Original `.env` text whose recognized inline comments should
///   be removed.
fn strip_dotenv_inline_comments(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut state = DotenvLineState::Complete;
    let mut in_comment = false;
    let mut physical_line_start = 0;

    for (index, character) in content.char_indices() {
        if index == physical_line_start
            && state == DotenvLineState::Complete
            && is_dotenv_full_comment_line(content, index)
        {
            state = DotenvLineState::Comment;
        }

        if !in_comment && state == DotenvLineState::WhiteSpace && character == '#' {
            in_comment = true;
        }

        if !in_comment || matches!(character, '\r' | '\n') {
            normalized.push(character);
        }
        state.advance(character);
        if character == '\n' {
            in_comment = false;
            physical_line_start = index + character.len_utf8();
        }
    }
    normalized
}

/// Protects substitutions independently within each dotenv logical line.
///
/// Dotenv uses one scanner to find logical-line boundaries and another to
/// parse value escapes. A backslash inside a single-quoted value escapes a
/// quote for boundary detection, while the value parser keeps its existing
/// literal backslash behavior. Resetting value state only at logical-line
/// boundaries prevents comment text from carrying quote state into the next
/// assignment.
/// # Parameters
///
/// * `content` - Input content being parsed.
fn escape_dotenv_substitutions(content: &str) -> String {
    let mut escaped_content = String::with_capacity(content.len());
    let mut state = DotenvLineState::Complete;
    let mut logical_line_start = 0;
    let mut physical_line_start = 0;

    for (index, character) in content.char_indices() {
        if index == physical_line_start
            && state == DotenvLineState::Complete
            && is_dotenv_full_comment_line(content, index)
        {
            state = DotenvLineState::Comment;
        }

        state.advance(character);
        let end = index + character.len_utf8();
        if character == '\n' && state == DotenvLineState::Complete {
            escaped_content.push_str(&escape_dotenv_substitutions_in_line(&content[logical_line_start..end]));
            logical_line_start = end;
            state = DotenvLineState::Complete;
        }
        if character == '\n' {
            physical_line_start = end;
        }
    }

    escaped_content.push_str(&escape_dotenv_substitutions_in_line(&content[logical_line_start..]));
    escaped_content
}

/// Returns whether the physical line beginning at `start` is a dotenv comment.
/// # Parameters
///
/// * `content` - Input content being parsed.
/// * `start` - Byte offset at which parsing begins.
fn is_dotenv_full_comment_line(content: &str, start: usize) -> bool {
    let line_end = content[start..]
        .find('\n')
        .map_or(content.len(), |offset| start + offset);
    content[start..line_end].trim_start().starts_with('#')
}

/// Escapes dotenv substitution markers while preserving quote and escape rules.
///
/// Strong-quoted backslashes remain ordinary value characters here. The
/// separate logical-line scanner handles escaped quotes for boundary
/// detection; this value state resets only after that scanner finds a line
/// boundary.
/// # Parameters
///
/// * `content` - Input content being parsed.
fn escape_dotenv_substitutions_in_line(content: &str) -> String {
    let mut escaped_content = String::with_capacity(content.len());
    let mut strong_quote = false;
    let mut weak_quote = false;
    let mut escaped = false;
    let mut comment = false;
    let mut in_value = false;
    let mut line_has_non_whitespace = false;
    let mut previous_was_whitespace = false;

    for character in content.chars() {
        if character == '\n' {
            escaped_content.push(character);
            comment = false;
            // The outer scanner owns logical assignment boundaries. A physical
            // newline inside this record must not reset the value phase, even
            // when value parsing and logical-line quote states have diverged.
            line_has_non_whitespace = false;
            previous_was_whitespace = false;
            escaped = false;
            continue;
        }

        if comment {
            escaped_content.push(character);
            continue;
        }

        if !in_value {
            escaped_content.push(character);
            if character == '=' {
                in_value = true;
            } else if character == '#' && !line_has_non_whitespace {
                comment = true;
            }
            if !character.is_whitespace() {
                line_has_non_whitespace = true;
            }
            previous_was_whitespace = is_dotenv_comment_separator(character);
            continue;
        }

        if escaped {
            escaped_content.push(character);
            escaped = false;
            line_has_non_whitespace |= !character.is_whitespace();
            // An escaped whitespace character belongs to the value, not a comment
            // separator.
            previous_was_whitespace = false;
            continue;
        }

        if strong_quote {
            escaped_content.push(character);
            if character == '\'' {
                strong_quote = false;
            }
            line_has_non_whitespace |= !character.is_whitespace();
            previous_was_whitespace = is_dotenv_comment_separator(character);
            continue;
        }

        if weak_quote {
            match character {
                '\\' => {
                    escaped_content.push(character);
                    escaped = true;
                }
                '"' => {
                    escaped_content.push(character);
                    weak_quote = false;
                }
                '$' => escaped_content.push_str("\\$"),
                _ => escaped_content.push(character),
            }
            line_has_non_whitespace |= !character.is_whitespace();
            previous_was_whitespace = is_dotenv_comment_separator(character);
            continue;
        }

        match character {
            '\'' => {
                strong_quote = true;
                escaped_content.push(character);
            }
            '"' => {
                weak_quote = true;
                escaped_content.push(character);
            }
            '\\' => {
                escaped = true;
                escaped_content.push(character);
            }
            '#' if previous_was_whitespace => {
                comment = true;
                escaped_content.push(character);
            }
            '$' => escaped_content.push_str("\\$"),
            _ => escaped_content.push(character),
        }
        line_has_non_whitespace |= !character.is_whitespace();
        previous_was_whitespace = is_dotenv_comment_separator(character);
    }

    escaped_content
}

/// Returns whether `character` separates an assignment value from a dotenv
/// comment.
///
/// Dotenv's logical-line scanner treats horizontal/Unicode whitespace as a
/// separator, but CR and LF terminate or participate in line parsing instead.
/// # Parameters
///
/// * `character` - Value used to character during this operation.
fn is_dotenv_comment_separator(character: char) -> bool {
    character.is_whitespace() && !matches!(character, '\r' | '\n')
}

impl EnvFileConfigSource {
    /// Creates a new `EnvFileConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.env` file.
    ///
    /// # Returns
    ///
    /// A source that reads the specified file.
    /// # Type Parameters
    ///
    /// * `P` - Path type that can be borrowed as a filesystem path.
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::builder().file(path).build()
    }

    /// Creates a dotenv source backed by in-memory text.
    ///
    /// # Parameters
    ///
    /// * `content` - The dotenv text to parse.
    ///
    /// # Returns
    ///
    /// A source that loads the supplied text.
    #[inline]
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::builder().content(content).build()
    }

    /// Creates a builder for a dotenv source.
    ///
    /// # Returns
    ///
    /// A builder with default limits and no source input.
    #[inline]
    pub fn builder() -> EnvFileConfigSourceBuilder {
        EnvFileConfigSourceBuilder::new()
    }
}

impl ConfigSource for EnvFileConfigSource {
    /// Returns the stable identifier used to report this source during loading.
    fn source_id(&self) -> String {
        self.label.clone().unwrap_or_else(|| self.input.label(".env"))
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
        let label = self.source_id();
        let content = self.input.read_to_string(".env", session)?;
        let content = strip_dotenv_inline_comments(&content);
        let content = escape_dotenv_substitutions(&content);
        let iter = dotenvy::from_read_iter(content.as_bytes());

        for item in iter {
            let (key, value) = item.map_err(|error| map_dotenv_error(&label, error))?;
            let _ = ConfigKey::parse(key.as_str())
                .map_err(|error| error.with_source_context(&label, Some(key.clone()), None))?;
            session.check_depth(key.split('.').count())?;
            session.consume_nodes(1)?;
            session.consume_properties(1)?;
            if let Err(error) = config.set(&key, value) {
                return Err(error.with_source_context(&label, Some(key), None));
            }
        }
        context.replace_layer(config);
        Ok(())
    }
}
