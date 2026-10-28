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

/// Scans YAML syntax tokens, rejecting anchors and aliases before the value
/// backend builds an AST and reporting whether the stream contains a value.
///
/// Scanner errors and unsupported tokens produce fixed, source-aware
/// diagnostics. Parser-owned text and token payloads are never included in an
/// error.
pub(in crate::source) fn inspect_yaml_tokens(label: &str, content: &str) -> ConfigResult<bool> {
    let mut scanner = Scanner::new(content.chars());
    let mut has_value_tokens = false;

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

#[cfg(test)]
mod tests {
    use super::inspect_yaml_tokens;
    use crate::ConfigError;

    #[test]
    fn accepts_yaml_without_anchor_tokens() {
        assert!(
            inspect_yaml_tokens("memory", "value: '*literal'\nquoted: \"&text\"\n")
                .expect("quoted anchor-like text is ordinary scalar content")
        );
    }

    #[test]
    fn rejects_alias_tokens_with_source_position() {
        let error = inspect_yaml_tokens("memory", "base: &secret value\ncopy: *secret\n")
            .expect_err("an alias token must be rejected before parsing values");

        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.to_string().contains("line 1, column 7"));
        assert!(error.to_string().contains("anchors and aliases are not supported"));
        assert!(!error.to_string().contains("secret"));
    }

    #[test]
    fn redacts_scanner_syntax_errors() {
        const SECRET: &str = "YAML_SCANNER_SECRET";
        let error = inspect_yaml_tokens("memory", &format!("value: \"{SECRET}\n"))
            .expect_err("an unterminated quoted scalar is invalid YAML");

        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert!(error.to_string().contains("invalid YAML syntax"));
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
    }

    #[test]
    fn reports_whether_a_document_contains_value_tokens() {
        for input in ["", " \n", "# comment\n", "---\n", "---\n# comment\n...\n"] {
            assert!(!inspect_yaml_tokens("memory", input).unwrap(), "input={input:?}");
        }
        for input in ["null", "[]", "{}"] {
            assert!(inspect_yaml_tokens("memory", input).unwrap(), "input={input:?}");
        }
    }
}
