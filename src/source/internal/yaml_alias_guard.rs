// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use yaml_rust2::scanner::Scanner;
use yaml_rust2::scanner::TokenType;

use self::collection_frame::CollectionFrame;
use super::text_location::text_location;
use crate::ConfigError;
use crate::ConfigResult;
use crate::SourceLocation;
use crate::source::source_load_session::SourceLoadSession;

mod collection_frame;

/// Scans YAML syntax tokens, rejecting anchors and aliases before the value
/// backend builds an AST and reporting whether the stream contains a value.
///
/// Scanner errors and unsupported tokens produce fixed, source-aware
/// diagnostics. Parser-owned text and token payloads are never included in an
/// error.
///
/// # Parameters
///
/// * `label` - The source label used in parser diagnostics.
/// * `content` - The YAML document whose tokens are inspected.
/// * `session` - The source-load budgets for parser tokens and nesting depth.
///
/// # Returns
///
/// `true` when the stream contains value tokens, including explicit nulls and
/// empty collections; otherwise, `false`.
///
/// # Errors
///
/// Returns a sanitized syntax error for scanner errors, unsupported anchors
/// or aliases, mismatched collection closers, or exceeded source limits.
pub(in crate::source) fn inspect_yaml_tokens(
    label: &str,
    content: &str,
    session: &mut SourceLoadSession<'_>,
) -> ConfigResult<bool> {
    let mut scanner = Scanner::new(content.chars());
    let mut has_value_tokens = false;
    let mut frames = Vec::with_capacity(8);

    while let Some(token) = scanner.next_token().map_err(|error| {
        let marker = error.marker();
        yaml_marker_error(
            label,
            content,
            marker.line(),
            marker.col(),
            format!("Failed to parse YAML file '{label}': invalid YAML syntax"),
        )
    })? {
        let yaml_rust2::scanner::Token(marker, token_type) = token;
        session.consume_parser_tokens(1)?;
        match &token_type {
            TokenType::BlockMappingStart => {
                admit_collection_depth(label, session, &mut frames, CollectionFrame::Block)?;
            }
            TokenType::BlockSequenceStart => {
                admit_collection_depth(label, session, &mut frames, CollectionFrame::Block)?;
            }
            TokenType::FlowMappingStart => {
                admit_collection_depth(label, session, &mut frames, CollectionFrame::FlowMapping)?;
            }
            TokenType::FlowSequenceStart => {
                admit_collection_depth(label, session, &mut frames, CollectionFrame::FlowSequence)?;
            }
            TokenType::BlockEnd => {
                close_collection(label, &mut frames, |frame| frame == CollectionFrame::Block)?;
            }
            TokenType::FlowMappingEnd => {
                close_collection(label, &mut frames, |frame| frame == CollectionFrame::FlowMapping)?;
            }
            TokenType::FlowSequenceEnd => {
                close_collection(label, &mut frames, |frame| frame == CollectionFrame::FlowSequence)?;
            }
            _ => {}
        }
        let is_value_token = match token_type {
            TokenType::Anchor(_) | TokenType::Alias(_) => {
                return Err(yaml_marker_error(
                    label,
                    content,
                    marker.line(),
                    marker.col(),
                    format!("YAML anchors and aliases are not supported in '{label}'"),
                ));
            }
            TokenType::StreamStart(_)
            | TokenType::StreamEnd
            | TokenType::VersionDirective(_, _)
            | TokenType::TagDirective(_, _)
            | TokenType::DocumentStart
            | TokenType::DocumentEnd => false,
            _ => true,
        };
        has_value_tokens |= is_value_token;
    }

    Ok(has_value_tokens)
}

/// Converts a one-based YAML marker line and zero-based Unicode scalar column
/// into the shared physical source location.
///
/// # Parameters
///
/// * `content` - The complete YAML source text.
/// * `line` - The one-based physical line reported by the scanner.
/// * `column` - The zero-based Unicode scalar column reported by the scanner.
///
/// The mapping runs only after a guard failure; successful scans do not rescan
/// the source once per token.
/// Returns `None` when the line is outside the input, the column exceeds the
/// physical line, or the resulting byte offset cannot be represented by the
/// shared location helper. A column at line end, including EOF, is valid.
fn yaml_marker_location(content: &str, line: usize, column: usize) -> Option<SourceLocation> {
    if line == 0 {
        return None;
    }
    let bytes = content.as_bytes();
    let mut line_number = 1usize;
    let mut line_start = 0usize;
    let mut cursor = 0usize;
    while line_number < line {
        if cursor >= bytes.len() {
            return None;
        }
        match bytes[cursor] {
            b'\r' => {
                cursor += if bytes.get(cursor + 1) == Some(&b'\n') { 2 } else { 1 };
                line_start = cursor;
                line_number += 1;
            }
            b'\n' => {
                cursor += 1;
                line_start = cursor;
                line_number += 1;
            }
            _ => cursor += 1,
        }
    }
    let mut line_end = line_start;
    while line_end < bytes.len() && !matches!(bytes[line_end], b'\r' | b'\n') {
        line_end += 1;
    }
    let physical_line = content.get(line_start..line_end)?;
    let offset = if column == physical_line.chars().count() {
        line_end
    } else {
        physical_line.char_indices().nth(column)?.0 + line_start
    };
    text_location(content, offset)
}

/// Creates a sanitized YAML error and attaches a marker location when valid.
///
/// # Parameters
///
/// * `label` - The diagnostic source identifier.
/// * `content` - The complete YAML source text used to map the marker.
/// * `line` - The one-based physical line reported by the scanner.
/// * `column` - The zero-based Unicode scalar column reported by the scanner.
/// * `message` - A fixed, sanitized failure reason.
fn yaml_marker_error(label: &str, content: &str, line: usize, column: usize, message: String) -> ConfigError {
    let location = yaml_marker_location(content, line, column);
    let suffix = location.map_or_else(String::new, |location| {
        format!(
            " at line {}, column {}",
            location.line(),
            location.column().unwrap_or(1)
        )
    });
    let error = ConfigError::source_parse_error(label, format!("{message}{suffix}"));
    match location {
        Some(location) => error.with_source_location(location),
        None => error,
    }
}

/// Checks and records one collection opening in the current YAML stream.
///
/// `frames` contains the still-open ancestors, so its length is the depth of
/// the collection being opened: the root collection has depth zero. The
/// depth limit is checked before the new frame is pushed; if admission fails,
/// `frames` is unchanged.
///
/// # Parameters
///
/// * `label` - The source label for a depth-limit error.
/// * `session` - The active source-load depth budget.
/// * `frames` - The open collection ancestors; updated after admission.
/// * `frame` - The collection kind to push when admitted.
///
/// # Returns
///
/// `Ok(())` after the frame is pushed.
///
/// # Errors
///
/// Returns the session's parser-depth limit error, augmented with `label`,
/// when this depth exceeds a local or aggregate parser-depth limit.
fn admit_collection_depth(
    label: &str,
    session: &SourceLoadSession<'_>,
    frames: &mut Vec<CollectionFrame>,
    frame: CollectionFrame,
) -> ConfigResult<()> {
    // The first collection is the YAML document root and has depth zero.
    let depth = frames.len();
    session
        .check_parser_depth(depth)
        .map_err(|error| error.with_source_context(label, None, None))?;
    frames.push(frame);
    Ok(())
}

/// Removes and validates the most recently opened collection.
///
/// The top frame is popped before its kind is checked. An empty stack or a
/// mismatched closing token returns the same sanitized YAML syntax diagnostic;
/// on a mismatch, the popped frame is not restored. Callers discard the
/// scanner's collection context after either error.
///
/// # Parameters
///
/// * `label` - The source label included in a syntax diagnostic.
/// * `frames` - The open collection stack, whose top entry is removed.
/// * `matches` - Predicate that accepts the expected closing collection kind.
///
/// # Returns
///
/// `Ok(())` when the popped frame matches the closing token.
///
/// # Errors
///
/// Returns [`invalid_collection_end`] when no collection is open or the most
/// recent opening does not match the closing token.
fn close_collection(
    label: &str,
    frames: &mut Vec<CollectionFrame>,
    matches: impl FnOnce(CollectionFrame) -> bool,
) -> ConfigResult<()> {
    let Some(frame) = frames.pop() else {
        return Err(invalid_collection_end(label));
    };
    if !matches(frame) {
        return Err(invalid_collection_end(label));
    }
    Ok(())
}

/// Creates the fixed diagnostic used for an invalid collection close.
///
/// The message contains the source label and generic syntax classification,
/// but never includes the closing token or parser-owned text.
///
/// # Parameters
///
/// * `label` - The source label to include in the diagnostic.
///
/// # Returns
///
/// A generic YAML syntax error that does not expose parser-owned text.
fn invalid_collection_end(label: &str) -> ConfigError {
    ConfigError::source_parse_error(
        label,
        format!("Failed to parse YAML file '{label}': invalid YAML syntax"),
    )
}

#[cfg(test)]
mod tests {
    use super::CollectionFrame;
    use super::close_collection;
    use super::yaml_marker_location;
    use crate::ConfigError;

    #[test]
    fn test_rejects_unbalanced_or_mismatched_collection_closes() {
        let mut frames = Vec::new();
        let error = close_collection("memory", &mut frames, |_| true)
            .expect_err("a close token without an open collection is invalid");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));

        frames.push(CollectionFrame::Block);
        let error = close_collection("memory", &mut frames, |frame| frame == CollectionFrame::FlowSequence)
            .expect_err("a close token must match its most recent opening token");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
    }

    #[test]
    fn test_yaml_marker_location_handles_physical_lines_and_bounds() {
        for text in ["a\né", "a\r\né", "a\ré"] {
            assert_eq!(
                yaml_marker_location(text, 2, 0).map(|location| (location.line(), location.column())),
                Some((2, Some(1)))
            );
            assert_eq!(
                yaml_marker_location(text, 2, 1).map(|location| (location.line(), location.column())),
                Some((2, Some(2)))
            );
        }
        assert!(yaml_marker_location("a", 0, 0).is_none());
        assert!(yaml_marker_location("a", 2, 0).is_none());
        assert!(yaml_marker_location("a", 1, 2).is_none());
        assert_eq!(
            yaml_marker_location("a", 1, 1).map(|location| (location.line(), location.column())),
            Some((1, Some(2)))
        );
        assert_eq!(
            yaml_marker_location("", 1, 0).map(|location| (location.line(), location.column())),
            Some((1, Some(1)))
        );
    }
}
