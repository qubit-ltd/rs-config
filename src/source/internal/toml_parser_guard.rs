// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Bounded TOML token admission and parser event depth checking.

use toml_parser::ParseError;
use toml_parser::Source;
use toml_parser::parser::parse_document;

use super::super::source_load_session::SourceLoadSession;
use super::toml_parser_depth::TomlParserDepth;
use crate::ConfigError;
use crate::ConfigResult;

/// Admits TOML parser work before the value-tree backend is entered.
///
/// Counts lexer tokens and checks parser nesting without constructing parser
/// values. The first depth or token-limit failure takes precedence over syntax
/// errors.
///
/// # Parameters
///
/// * `label` - The source label used in diagnostic errors.
/// * `content` - The TOML document to lex and inspect.
/// * `session` - The source-load budgets used to admit parser work.
///
/// # Returns
///
/// `Ok(())` when token, depth, and syntax checks succeed.
///
/// # Errors
///
/// Returns a source-limit error when token or depth limits are exceeded, or a
/// parse error when the document is invalid TOML.
pub(super) fn admit_toml_parser(label: &str, content: &str, session: &mut SourceLoadSession<'_>) -> ConfigResult<()> {
    let mut tokens = Vec::with_capacity(16);
    for token in Source::new(content).lex() {
        session.consume_parser_tokens(1)?;
        tokens.push(token);
    }

    let mut receiver = TomlParserDepth::new(session);
    let mut parser_error: Option<ParseError> = None;
    parse_document(&tokens, &mut receiver, &mut parser_error);
    drop(tokens);

    receiver.finish()?;
    if let Some(error) = parser_error {
        let location = error
            .context()
            .or(error.unexpected())
            .map(|span| toml_location(content, span.start()))
            .unwrap_or_default();
        return Err(ConfigError::source_parse_error(
            label,
            format!("Invalid TOML syntax{location}"),
        ));
    }
    Ok(())
}

/// Checks TOML parser limits before invoking the value-tree backend.
///
/// The backend is called only after parser token, depth, and syntax checks
/// succeed.
///
/// # Type Parameters
///
/// * `T` - The backend result type.
///
/// # Parameters
///
/// * `label` - The source label used in diagnostic errors.
/// * `content` - The TOML document to admit before backend parsing.
/// * `session` - The source-load budgets used to admit parser work.
/// * `backend` - The value-tree parser run after admission succeeds.
///
/// # Returns
///
/// The backend result after parser admission succeeds.
///
/// # Errors
///
/// Returns an admission error or the error returned by the backend.
pub(in crate::source) fn with_toml_admission<T>(
    label: &str,
    content: &str,
    session: &mut SourceLoadSession<'_>,
    backend: impl FnOnce() -> ConfigResult<T>,
) -> ConfigResult<T> {
    admit_toml_parser(label, content, session)?;
    backend()
}

/// Formats a byte offset as a one-based TOML line and column location.
///
/// Returns an empty string when the offset is not a UTF-8 character boundary.
///
/// # Parameters
///
/// * `content` - The source text containing the offset.
/// * `offset` - The byte offset to convert.
///
/// # Returns
///
/// A formatted location suffix, or an empty string for an invalid offset.
fn toml_location(content: &str, offset: usize) -> String {
    let Some(prefix) = content.get(..offset) else {
        return String::new();
    };
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    format!(" at line {line}, column {column}")
}

#[cfg(test)]
mod tests {
    use super::admit_toml_parser;
    use super::toml_location;
    use crate::ConfigError;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn test_invalid_backend_offset_has_no_location() {
        assert_eq!(toml_location("value", usize::MAX), "");

        let mut session = SourceLoadSession::new("TOML:test", SourceLimits::unbounded());
        let error = admit_toml_parser("TOML:test", "value = [1", &mut session)
            .expect_err("the parser must reject an unterminated TOML array");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
    }
}
