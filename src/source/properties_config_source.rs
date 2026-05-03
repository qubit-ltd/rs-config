/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
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

use std::path::{Path, PathBuf};

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

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
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// let value = config.get::<String>("server.port").unwrap();
/// assert_eq!(value, "8080");
/// ```
///
#[derive(Debug, Clone)]
pub struct PropertiesConfigSource {
    path: PathBuf,
}

impl PropertiesConfigSource {
    /// Creates a new `PropertiesConfigSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.properties` file
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Parses a `.properties` format string into key-value pairs
    ///
    /// # Parameters
    ///
    /// * `content` - The content of the `.properties` file
    ///
    /// # Returns
    ///
    /// Returns a vector of `(key, value)` pairs
    pub fn parse_content(content: &str) -> Vec<(String, String)> {
        let mut result = Vec::new();
        let mut lines = content.lines().peekable();

        while let Some(line) = lines.next() {
            let trimmed = line.trim_start();

            // Skip blank lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                continue;
            }

            // Handle line continuation
            let mut full_line = trimmed.to_string();
            while has_line_continuation(&full_line) {
                full_line.pop(); // remove trailing backslash
                if let Some(next) = lines.next() {
                    full_line.push_str(next.trim_start());
                } else {
                    break;
                }
            }

            // Parse key/value pairs using Java properties separators.
            if let Some((key, value)) = parse_key_value(&full_line) {
                let key = unescape_properties(key);
                let value = unescape_properties(value);
                result.push((key, value));
            }
        }

        result
    }
}

/// Parses a single `key=value`, `key: value`, or `key value` line.
fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();

    for (i, ch) in line.char_indices() {
        if ch == '=' || ch == ':' {
            // Separator is escaped only if there is an odd number of trailing backslashes.
            if !is_escaped_separator(line, i) {
                let value_start = skip_properties_whitespace(line, i + ch.len_utf8());
                return Some((&line[..i], &line[value_start..]));
            }
        }
        if ch.is_whitespace() && !is_escaped_separator(line, i) {
            let mut value_start = skip_properties_whitespace(line, i);
            if let Some((sep, sep_len)) = char_at(line, value_start)
                && (sep == '=' || sep == ':')
                && !is_escaped_separator(line, value_start)
            {
                value_start = skip_properties_whitespace(line, value_start + sep_len);
            }
            return Some((&line[..i], &line[value_start..]));
        }
    }
    // No separator found - treat the whole line as a key with empty value.
    (!line.is_empty()).then_some((line, ""))
}

/// Returns the character and byte width at `index`.
///
/// # Parameters
///
/// * `line` - Source properties line.
/// * `index` - Byte index to inspect.
///
/// # Returns
///
/// `Some((ch, len))` if `index` points to a character boundary inside `line`,
/// otherwise `None`.
#[inline]
fn char_at(line: &str, index: usize) -> Option<(char, usize)> {
    if index == line.len() {
        return None;
    }
    let ch = line[index..]
        .chars()
        .next()
        .expect("index below line length should point to a character");
    Some((ch, ch.len_utf8()))
}

/// Skips Java properties whitespace from a byte index.
///
/// # Parameters
///
/// * `line` - Source properties line.
/// * `start` - Byte index to start scanning from.
///
/// # Returns
///
/// The first byte index at or after `start` that is not whitespace, or the end
/// of `line`.
fn skip_properties_whitespace(line: &str, start: usize) -> usize {
    for (offset, ch) in line[start..].char_indices() {
        if !ch.is_whitespace() {
            return start + offset;
        }
    }
    line.len()
}

/// Returns true if the separator at `sep_pos` is escaped by a preceding odd
/// number of backslashes.
///
/// # Parameters
///
/// * `line` - Full properties line being parsed.
/// * `sep_pos` - Byte index of `=` or `:` in `line`.
///
/// # Returns
///
/// `true` when the separator is escaped and must not split the key/value.
#[inline]
fn is_escaped_separator(line: &str, sep_pos: usize) -> bool {
    let slash_count = line.as_bytes()[..sep_pos]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count();
    slash_count % 2 == 1
}

/// Returns true if a physical line continues on the next line.
///
/// Java-style properties only treat an odd number of trailing backslashes as a
/// continuation marker; an even number represents escaped literal backslashes.
///
/// # Parameters
///
/// * `line` - Physical properties line after outer whitespace trimming.
///
/// # Returns
///
/// `true` when the line should be joined with the next physical line.
#[inline]
fn has_line_continuation(line: &str) -> bool {
    count_trailing_backslashes(line) % 2 == 1
}

/// Counts consecutive trailing backslashes in a string.
///
/// # Parameters
///
/// * `line` - Source line or key/value segment.
///
/// # Returns
///
/// Number of trailing `\` bytes.
#[inline]
fn count_trailing_backslashes(line: &str) -> usize {
    line.as_bytes()
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
}

/// Processes Java properties escape sequences in a string.
fn unescape_properties(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars.next().unwrap_or('\\');
            match escaped {
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4
                        && let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(unicode_char) = char::from_u32(code)
                    {
                        result.push(unicode_char);
                        continue;
                    }
                    // If parsing fails, keep original
                    result.push('\\');
                    result.push('u');
                    result.push_str(&hex);
                }
                'n' => {
                    result.push('\n');
                }
                't' => {
                    result.push('\t');
                }
                'r' => {
                    result.push('\r');
                }
                'f' => {
                    result.push('\u{000C}');
                }
                '\\' => {
                    result.push('\\');
                }
                '=' | ':' | ' ' | '#' | '!' => {
                    result.push(escaped);
                }
                _ => {
                    result.push(escaped);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

impl ConfigSource for PropertiesConfigSource {
    fn load(&self, config: &mut Config) -> ConfigResult<()> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read properties file '{}': {}",
                    self.path.display(),
                    e
                ),
            ))
        })?;

        let mut staged = config.clone();
        for (key, value) in Self::parse_content(&content) {
            staged.set(&key, value)?;
        }

        *config = staged;
        Ok(())
    }
}
