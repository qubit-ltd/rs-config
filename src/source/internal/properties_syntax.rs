// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Java properties line scanning, separator parsing, and escape decoding.

use std::iter::Peekable;
use std::iter::from_fn;
use std::str::Chars;

/// Borrows physical lines while preserving empty lines and treating CRLF as
/// one terminator. Empty input and a final terminator add no synthetic line.
///
/// # Parameters
///
/// * `remaining` - Properties text whose physical lines are traversed.
///
/// # Returns
///
/// An iterator over borrowed physical lines without a synthetic trailing line.
#[inline]
pub(in crate::source) fn properties_natural_lines(mut remaining: &str) -> impl Iterator<Item = &str> {
    from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let end = remaining.find(['\r', '\n']).unwrap_or(remaining.len());
        let line = &remaining[..end];
        let suffix = &remaining[end..];
        remaining = suffix
            .strip_prefix("\r\n")
            .or_else(|| suffix.strip_prefix('\r'))
            .or_else(|| suffix.strip_prefix('\n'))
            .unwrap_or(suffix);
        Some(line)
    })
}

/// Returns the byte offset at the start of a one-based physical properties
/// line.
///
/// # Parameters
/// - `content`: Properties source text whose physical lines are counted.
/// - `target_line`: One-based line number to locate.
///
/// # Returns
/// The byte offset at the start of `target_line`, or the end of `content` if
/// the requested line is past the source.
pub(in crate::source) fn properties_line_offset(content: &str, target_line: usize) -> usize {
    let mut line = 1usize;
    let bytes = content.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && line < target_line {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                index += 2;
                line += 1;
            }
            b'\r' | b'\n' => {
                index += 1;
                line += 1;
            }
            _ => index += 1,
        }
    }
    index
}

/// Parses a single `key=value`, `key: value`, or `key value` line.
///
/// # Parameters
///
/// * `line` - One trimmed properties line.
///
/// # Returns
///
/// The borrowed key and value split at the first unescaped separator. If a
/// non-empty line has no separator, the whole line is the key and the value is
/// empty. Returns `None` only when `line` is empty after trimming.
pub(in crate::source) fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start_matches(is_properties_whitespace);

    for (i, ch) in line.char_indices() {
        if ch == '=' || ch == ':' {
            // Separator is escaped only if there is an odd number of trailing
            // backslashes.
            if !is_escaped_separator(line, i) {
                let value_start = skip_properties_whitespace(line, i + ch.len_utf8());
                return Some((&line[..i], &line[value_start..]));
            }
        }
        if is_properties_whitespace(ch) && !is_escaped_separator(line, i) {
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
/// * `index` - Byte index to inspect. It must be at most `line.len()` and lie
///   on a UTF-8 character boundary.
///
/// # Returns
///
/// `Some((ch, len))` if `index` points inside `line` at a character boundary;
/// `None` if `index == line.len()`.
///
/// # Panics
///
/// Panics if `index > line.len()` or if `index` is not a UTF-8 character
/// boundary.
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
///
/// # Panics
///
/// Panics if `start` is past the end of `line` or is not a UTF-8 character
/// boundary.
fn skip_properties_whitespace(line: &str, start: usize) -> usize {
    for (offset, ch) in line[start..].char_indices() {
        if !is_properties_whitespace(ch) {
            return start + offset;
        }
    }
    line.len()
}

/// Identifies the whitespace characters used by Java properties separators.
///
/// # Parameters
///
/// * `ch` - The character to classify.
///
/// # Returns
///
/// `true` for space, tab, or form feed; otherwise, `false`.
#[inline]
pub(in crate::source) fn is_properties_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000c}')
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
///
/// # Panics
///
/// Panics if `sep_pos` is past the end of `line`.
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
pub(in crate::source) fn has_line_continuation(line: &str) -> bool {
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
    line.as_bytes().iter().rev().take_while(|&&b| b == b'\\').count()
}

/// Processes Java properties escape sequences in a string.
///
/// Valid Unicode escapes decode UTF-16 code units and valid surrogate pairs;
/// malformed Unicode escapes are preserved verbatim, while unknown escapes
/// discard the leading backslash.
///
/// # Parameters
///
/// * `s` - The properties key or value to unescape.
///
/// # Returns
///
/// An owned string with valid escapes decoded.
pub(in crate::source) fn unescape_properties(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars.next().unwrap_or('\\');
            match escaped {
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4
                        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                        && let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(unicode_char) = decode_unicode_escape(code, &mut chars)
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

/// Decodes a Java properties `\uXXXX` escape, including UTF-16 surrogate pairs.
///
/// # Parameters
///
/// * `code` - The decoded UTF-16 code unit.
/// * `chars` - Remaining escape input, advanced when a valid surrogate pair is
///   consumed.
///
/// # Returns
///
/// The decoded Unicode scalar, or `None` for a lone surrogate or invalid
/// scalar.
fn decode_unicode_escape(code: u32, chars: &mut Peekable<Chars<'_>>) -> Option<char> {
    if is_high_surrogate(code) {
        let mut lookahead = chars.clone();
        if lookahead.next() == Some('\\') && lookahead.next() == Some('u') {
            let low_hex: String = lookahead.by_ref().take(4).collect();
            if low_hex.len() == 4
                && low_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                && let Ok(low) = u32::from_str_radix(&low_hex, 16)
                && is_low_surrogate(low)
            {
                *chars = lookahead;
                return decode_surrogate_pair(code, low);
            }
        }
        None
    } else if is_low_surrogate(code) {
        None
    } else {
        char::from_u32(code)
    }
}

/// Returns whether `code` is a UTF-16 high surrogate.
///
/// # Parameters
///
/// * `code` - The UTF-16 code unit to classify.
///
/// # Returns
///
/// `true` when `code` is in the high-surrogate range.
#[inline]
fn is_high_surrogate(code: u32) -> bool {
    (0xD800..=0xDBFF).contains(&code)
}

/// Returns whether `code` is a UTF-16 low surrogate.
///
/// # Parameters
///
/// * `code` - The UTF-16 code unit to classify.
///
/// # Returns
///
/// `true` when `code` is in the low-surrogate range.
#[inline]
fn is_low_surrogate(code: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&code)
}

/// Decodes a UTF-16 surrogate pair to a Unicode scalar value.
///
/// # Parameters
///
/// * `high` - The leading code unit; it must be a high surrogate.
/// * `low` - The following code unit; it must be a low surrogate.
///
/// # Returns
///
/// The Unicode scalar represented by the pair.
///
/// # Panics
///
/// Panics if either code unit is outside its corresponding surrogate range.
#[inline]
fn decode_surrogate_pair(high: u32, low: u32) -> Option<char> {
    let scalar = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
    char::from_u32(scalar)
}
