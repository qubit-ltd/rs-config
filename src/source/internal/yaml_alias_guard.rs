// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use yaml_rust2::scanner::Scanner;
use yaml_rust2::scanner::TokenType;

use crate::ConfigError;
use crate::ConfigResult;
use crate::source::source_load_session::SourceLoadSession;

/// Scans YAML syntax tokens, rejecting anchors and aliases before the value
/// backend builds an AST and reporting whether the stream contains a value.
///
/// Scanner errors and unsupported tokens produce fixed, source-aware
/// diagnostics. Parser-owned text and token payloads are never included in an
/// error.
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
        ConfigError::source_parse_error(
            label,
            format!(
                "Failed to parse YAML file '{label}' at line {}, column {}: invalid YAML syntax",
                marker.line(),
                marker.col() + 1,
            ),
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
                return Err(ConfigError::source_parse_error(
                    label,
                    format!(
                        "YAML anchors and aliases are not supported in '{label}' at line {}, column {}",
                        marker.line(),
                        marker.col() + 1,
                    ),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollectionFrame {
    Block,
    FlowMapping,
    FlowSequence,
}

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
    use super::inspect_yaml_tokens;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    fn session() -> SourceLoadSession<'static> {
        SourceLoadSession::new("memory", SourceLimits::unbounded())
    }
    use crate::ConfigError;

    #[test]
    fn accepts_yaml_without_anchor_tokens() {
        assert!(
            inspect_yaml_tokens("memory", "value: '*literal'\nquoted: \"&text\"\n", &mut session())
                .expect("quoted anchor-like text is ordinary scalar content")
        );
    }

    #[test]
    fn rejects_alias_tokens_with_source_position() {
        let error = inspect_yaml_tokens("memory", "base: &secret value\ncopy: *secret\n", &mut session())
            .expect_err("an alias token must be rejected before parsing values");

        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.to_string().contains("line 1, column 7"));
        assert!(error.to_string().contains("anchors and aliases are not supported"));
        assert!(!error.to_string().contains("secret"));
    }

    #[test]
    fn redacts_scanner_syntax_errors() {
        const SECRET: &str = "YAML_SCANNER_SECRET";
        let error = inspect_yaml_tokens("memory", &format!("value: \"{SECRET}\n"), &mut session())
            .expect_err("an unterminated quoted scalar is invalid YAML");

        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.to_string().contains("invalid YAML syntax"));
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
    }

    #[test]
    fn reports_whether_a_document_contains_value_tokens() {
        for input in ["", " \n", "# comment\n", "---\n", "---\n# comment\n...\n"] {
            assert!(
                !inspect_yaml_tokens("memory", input, &mut session()).unwrap(),
                "input={input:?}"
            );
        }
        for input in ["null", "[]", "{}"] {
            assert!(
                inspect_yaml_tokens("memory", input, &mut session()).unwrap(),
                "input={input:?}"
            );
        }
    }

    #[test]
    fn rejects_unbalanced_or_mismatched_collection_closes() {
        let mut frames = Vec::new();
        let error = close_collection("memory", &mut frames, |_| true)
            .expect_err("a close token without an open collection is invalid");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));

        frames.push(CollectionFrame::Block);
        let error = close_collection("memory", &mut frames, |frame| frame == CollectionFrame::FlowSequence)
            .expect_err("a close token must match its most recent opening token");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
    }
}
