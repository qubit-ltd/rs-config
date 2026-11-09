// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Converts byte offsets in source text to one-based physical coordinates.

use std::num::NonZeroUsize;

use crate::SourceLocation;

/// Converts a UTF-8 byte offset to a one-based physical line and scalar column.
///
/// CRLF is a single line ending. An offset between its bytes belongs to the
/// previous line end; the offset after it starts the next line at column 1.
///
/// # Parameters
/// - `content`: UTF-8 source text to measure.
/// - `offset`: Byte offset to convert.
///
/// # Returns
/// The source location when the offset is in range and on a character
/// boundary; otherwise, `None`.
pub(crate) fn text_location(content: &str, offset: usize) -> Option<SourceLocation> {
    if offset > content.len() || !content.is_char_boundary(offset) {
        return None;
    }
    let bytes = content.as_bytes();
    let mut line = 1usize;
    let mut column = 1usize;
    let mut index = 0usize;
    while index < offset {
        match bytes[index] {
            b'\r' => {
                if bytes.get(index + 1) == Some(&b'\n') {
                    if index + 1 >= offset {
                        break;
                    }
                    index += 2;
                } else {
                    index += 1;
                }
                line += 1;
                column = 1;
            }
            b'\n' => {
                index += 1;
                line += 1;
                column = 1;
            }
            _ => {
                let ch = content[index..].chars().next()?;
                index += ch.len_utf8();
                column += 1;
            }
        }
    }
    Some(SourceLocation::new(NonZeroUsize::new(line)?, NonZeroUsize::new(column)))
}

#[cfg(test)]
mod tests {
    use super::text_location;

    #[test]
    fn test_counts_unicode_tabs_and_eof() {
        assert_eq!(text_location("a\té", 0).unwrap().column(), Some(1));
        assert_eq!(text_location("a\té", 2).unwrap().column(), Some(3));
        assert_eq!(text_location("a\té", 4).unwrap().column(), Some(4));
    }

    #[test]
    fn test_counts_crlf_cr_and_lf() {
        let text = "ab\r\ncd\ref\ng";
        assert_eq!(
            (
                text_location(text, 3).unwrap().line(),
                text_location(text, 3).unwrap().column()
            ),
            (1, Some(3))
        );
        assert_eq!(
            (
                text_location(text, 4).unwrap().line(),
                text_location(text, 4).unwrap().column()
            ),
            (2, Some(1))
        );
        assert_eq!(
            (
                text_location(text, 7).unwrap().line(),
                text_location(text, 7).unwrap().column()
            ),
            (3, Some(1))
        );
        assert_eq!(
            (
                text_location(text, text.len()).unwrap().line(),
                text_location(text, text.len()).unwrap().column()
            ),
            (4, Some(2))
        );
        assert_eq!(
            (
                text_location("", 0).unwrap().line(),
                text_location("", 0).unwrap().column()
            ),
            (1, Some(1))
        );
    }

    #[test]
    fn test_rejects_invalid_offsets() {
        assert!(text_location("é", 1).is_none());
        assert!(text_location("é", 3).is_none());
    }
}
