/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
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
//! - `# comment` and `! comment` lines
//! - Blank lines (ignored)
//! - Line continuation with `\` at end of line
//! - Unicode escape sequences (`\uXXXX`)
//!
//! # Author
//!
//! Haixing Hu

use std::path::{Path, PathBuf};

use crate::{Config, ConfigError, ConfigResult};

use super::ConfigSource;

/// Configuration source that loads from Java `.properties` format files
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::source::{PropertiesSource, ConfigSource};
/// use qubit_config::Config;
///
/// let source = PropertiesSource::from_file("config.properties");
/// let mut config = Config::new();
/// source.load(&mut config).unwrap();
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct PropertiesSource {
    path: PathBuf,
}

impl PropertiesSource {
    /// Creates a new `PropertiesSource` from a file path
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the `.properties` file
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
            let trimmed = line.trim();

            // Skip blank lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                continue;
            }

            // Handle line continuation
            let mut full_line = trimmed.to_string();
            while full_line.ends_with('\\') {
                full_line.pop(); // remove trailing backslash
                if let Some(next) = lines.next() {
                    full_line.push_str(next.trim());
                } else {
                    break;
                }
            }

            // Parse key=value or key: value
            if let Some((key, value)) = parse_key_value(&full_line) {
                let key = unescape_unicode(key.trim());
                let value = unescape_unicode(value.trim());
                result.push((key, value));
            }
        }

        result
    }
}

/// Parses a single `key=value` or `key: value` line
fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    // Find the first '=' or ':' that is not preceded by '\'
    let chars = line.char_indices();
    for (i, ch) in chars {
        if ch == '=' || ch == ':' {
            // Separator is escaped only if there is an odd number of trailing backslashes.
            if !is_escaped_separator(line, i) {
                return Some((&line[..i], &line[i + ch.len_utf8()..]));
            }
        }
    }
    // No separator found - treat the whole line as a key with empty value
    if !line.is_empty() {
        Some((line, ""))
    } else {
        None
    }
}

/// Returns true if the separator at `sep_pos` is escaped by a preceding odd number of backslashes.
fn is_escaped_separator(line: &str, sep_pos: usize) -> bool {
    let slash_count = line.as_bytes()[..sep_pos]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count();
    slash_count % 2 == 1
}

/// Processes Unicode escape sequences (`\uXXXX`) in a string
fn unescape_unicode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('u') => {
                    chars.next(); // consume 'u'
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(unicode_char) = char::from_u32(code) {
                                result.push(unicode_char);
                                continue;
                            }
                        }
                    }
                    // If parsing fails, keep original
                    result.push('\\');
                    result.push('u');
                    result.push_str(&hex);
                }
                Some('n') => {
                    chars.next();
                    result.push('\n');
                }
                Some('t') => {
                    chars.next();
                    result.push('\t');
                }
                Some('r') => {
                    chars.next();
                    result.push('\r');
                }
                Some('\\') => {
                    chars.next();
                    result.push('\\');
                }
                _ => {
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

impl ConfigSource for PropertiesSource {
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

        for (key, value) in Self::parse_content(&content) {
            config.set(&key, value)?;
        }

        Ok(())
    }
}
