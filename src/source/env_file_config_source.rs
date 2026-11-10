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

use std::borrow::Cow;
use std::path::Path;

use super::ConfigSource;
use super::SourceLimits;
use super::SourceLoadContext;
use super::internal::dotenv_line_state::DotenvLineState;
use super::source_input::SourceInput;
use crate::Config;
use crate::ConfigError;
use crate::ConfigResult;
use crate::config_path::ensure_config_key;

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
fn strip_dotenv_inline_comments(content: &str) -> Cow<'_, str> {
    let mut state = DotenvLineState::Complete;
    let mut in_comment = false;
    let mut physical_line_start = 0;
    let mut characters = content.char_indices();

    while let Some((index, character)) = characters.next() {
        if index == physical_line_start
            && state == DotenvLineState::Complete
            && is_dotenv_full_comment_line(content, index)
        {
            state = DotenvLineState::Comment;
        }

        if !in_comment && state == DotenvLineState::WhiteSpace && character == '#' {
            in_comment = true;
        }

        if in_comment && !matches!(character, '\r' | '\n') {
            let mut normalized = String::with_capacity(content.len());
            normalized.push_str(&content[..index]);
            state.advance(character);
            append_dotenv_inline_comments_remainder(
                content,
                characters,
                &mut state,
                &mut in_comment,
                &mut physical_line_start,
                &mut normalized,
            );
            return Cow::Owned(normalized);
        }
        state.advance(character);
        if character == '\n' {
            in_comment = false;
            physical_line_start = index + character.len_utf8();
        }
    }
    Cow::Borrowed(content)
}

/// Appends the rest of a comment-normalized input after its first removal.
///
/// The caller has advanced `state` through the first removed character and
/// copied the unchanged prefix, so this loop can append retained characters
/// directly without checking whether an output buffer exists on each step.
///
/// # Parameters
///
/// * `content` - Full input used to identify physical comment lines.
/// * `characters` - Remaining characters after the first removed character.
/// * `state` - Dotenv line state after that removed character.
/// * `in_comment` - Whether the current physical line is in an inline comment.
/// * `physical_line_start` - Byte offset of the current physical line start.
/// * `output` - Buffer receiving retained characters.
fn append_dotenv_inline_comments_remainder(
    content: &str,
    characters: std::str::CharIndices<'_>,
    state: &mut DotenvLineState,
    in_comment: &mut bool,
    physical_line_start: &mut usize,
    output: &mut String,
) {
    for (index, character) in characters {
        if index == *physical_line_start
            && *state == DotenvLineState::Complete
            && is_dotenv_full_comment_line(content, index)
        {
            *state = DotenvLineState::Comment;
        }

        if !*in_comment && *state == DotenvLineState::WhiteSpace && character == '#' {
            *in_comment = true;
        }

        if !*in_comment || matches!(character, '\r' | '\n') {
            output.push(character);
        }
        state.advance(character);
        if character == '\n' {
            *in_comment = false;
            *physical_line_start = index + character.len_utf8();
        }
    }
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
fn escape_dotenv_substitutions(content: &str) -> Cow<'_, str> {
    if !content.contains('$') {
        return Cow::Borrowed(content);
    }

    let mut escaped_content = None;
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
            escape_dotenv_substitutions_in_line(content, logical_line_start, end, &mut escaped_content);
            logical_line_start = end;
            state = DotenvLineState::Complete;
        }
        if character == '\n' {
            physical_line_start = end;
        }
    }

    escape_dotenv_substitutions_in_line(content, logical_line_start, content.len(), &mut escaped_content);
    escaped_content.map_or(Cow::Borrowed(content), Cow::Owned)
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
fn escape_dotenv_substitutions_in_line(content: &str, start: usize, end: usize, escaped_content: &mut Option<String>) {
    let mut strong_quote = false;
    let mut weak_quote = false;
    let mut escaped = false;
    let mut comment = false;
    let mut in_value = false;
    let mut line_has_non_whitespace = false;
    let mut previous_was_whitespace = false;

    let mut copied_through = start;
    for (relative_index, character) in content[start..end].char_indices() {
        if character == '\n' {
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
            continue;
        }

        if !in_value {
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
            escaped = false;
            line_has_non_whitespace |= !character.is_whitespace();
            // An escaped whitespace character belongs to the value, not a comment
            // separator.
            previous_was_whitespace = false;
            continue;
        }

        if strong_quote {
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
                    escaped = true;
                }
                '"' => {
                    weak_quote = false;
                }
                '$' => {
                    let index = start + relative_index;
                    escape_dotenv_dollar(content, index, index + 1, &mut copied_through, escaped_content);
                }
                _ => {}
            }
            line_has_non_whitespace |= !character.is_whitespace();
            previous_was_whitespace = is_dotenv_comment_separator(character);
            continue;
        }

        match character {
            '\'' => {
                strong_quote = true;
            }
            '"' => {
                weak_quote = true;
            }
            '\\' => {
                escaped = true;
            }
            '#' if previous_was_whitespace => {
                comment = true;
            }
            '$' => {
                let index = start + relative_index;
                escape_dotenv_dollar(content, index, index + 1, &mut copied_through, escaped_content);
            }
            _ => {}
        }
        line_has_non_whitespace |= !character.is_whitespace();
        previous_was_whitespace = is_dotenv_comment_separator(character);
    }

    if let Some(output) = escaped_content {
        output.push_str(&content[copied_through..end]);
    }
}

/// Copies the unchanged prefix and escapes one substitution marker on demand.
///
/// The first call allocates the shared output and copies the source prefix up
/// to `index`; later calls append only the bytes since the previous marker.
/// This lets callers keep borrowing `content` when no marker needs protection.
///
/// # Parameters
///
/// * `content` - Complete preprocessed content being scanned.
/// * `index` - Global byte offset of the dollar sign.
/// * `end` - Global byte offset immediately after the dollar sign.
/// * `copied_through` - End offset of the last source bytes copied to output.
/// * `output` - Shared lazily allocated output, if a marker has been escaped.
fn escape_dotenv_dollar(
    content: &str,
    index: usize,
    end: usize,
    copied_through: &mut usize,
    output: &mut Option<String>,
) {
    let first_change = output.is_none();
    let escaped = output.get_or_insert_with(|| String::with_capacity(content.len()));
    if first_change {
        escaped.push_str(&content[..index]);
    } else {
        escaped.push_str(&content[*copied_through..index]);
    }
    escaped.push_str("\\$");
    *copied_through = end;
}

/// Returns whether `character` separates an assignment value from a dotenv
/// comment.
///
/// Dotenv's logical-line scanner treats horizontal/Unicode whitespace as a
/// separator, but CR and LF terminate or participate in line parsing instead.
/// # Parameters
///
/// * `character` - Character being checked as a separator between a value and
///   an inline comment. Carriage return and line feed are not horizontal
///   separators here.
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
            ensure_config_key(key.as_str())
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

#[cfg(test)]
mod preprocessing_tests {
    use std::borrow::Cow;

    use super::escape_dotenv_substitutions;
    use super::strip_dotenv_inline_comments;

    #[test]
    fn test_comment_stripping_borrows_unmodified_input_and_owns_removed_comments() {
        for input in ["", "KEY=value\n", "# whole line\n", "KEY='a # b'\n", "KEY=\\$NAME\n"] {
            assert!(matches!(strip_dotenv_inline_comments(input), Cow::Borrowed(_)));
        }

        let input = "KEY=value # remove\r\nNEXT=ok";
        assert_eq!(strip_dotenv_inline_comments(input).as_ref(), "KEY=value \r\nNEXT=ok");
        assert!(matches!(strip_dotenv_inline_comments(input), Cow::Owned(_)));

        let unicode = "VALUE=你好 # remove";
        assert_eq!(strip_dotenv_inline_comments(unicode).as_ref(), "VALUE=你好 ");

        let later_unicode_comment = "PREFIX=你好\nVALUE=x # remove\nLAST=y # trim";
        assert_eq!(
            strip_dotenv_inline_comments(later_unicode_comment).as_ref(),
            "PREFIX=你好\nVALUE=x \nLAST=y "
        );
    }

    #[test]
    fn test_substitution_escaping_borrows_unmodified_and_owns_first_and_later_changes() {
        for input in ["", "KEY=value\n", "# ${NAME}\n", "KEY='${NAME}'\n", "KEY=\\$NAME\n"] {
            assert!(matches!(escape_dotenv_substitutions(input), Cow::Borrowed(_)));
        }

        for (input, expected) in [
            ("KEY=${NAME}\n", "KEY=\\${NAME}\n"),
            ("FIRST=ok\nLAST=${NAME}", "FIRST=ok\nLAST=\\${NAME}"),
            ("PREFIX=你好\nLAST=${NAME}", "PREFIX=你好\nLAST=\\${NAME}"),
            ("FIRST=ok\nLAST=${名}", "FIRST=ok\nLAST=\\${名}"),
            ("A=${A}\r\nB=${B}\r\n", "A=\\${A}\r\nB=\\${B}\r\n"),
            ("A=${A}\nB=${B}\n", "A=\\${A}\nB=\\${B}\n"),
            ("A=${A}\n", "A=\\${A}\n"),
        ] {
            let output = escape_dotenv_substitutions(input);
            assert!(matches!(output, Cow::Owned(_)), "input: {input:?}");
            assert_eq!(output, expected, "input: {input:?}");
            assert_eq!(output.matches("\\$").count(), input.matches('$').count());
        }
    }
}
