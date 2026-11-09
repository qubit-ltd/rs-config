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
use super::text_location::text_location;
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
            .and_then(|span| text_location(content, span.start()));
        let suffix = location.map_or_else(String::new, |location| {
            format!(
                " at line {}, column {}",
                location.line(),
                location.column().unwrap_or(1)
            )
        });
        let error = ConfigError::source_parse_error(label, format!("Invalid TOML syntax{suffix}"));
        return Err(match location {
            Some(location) => error.with_source_location(location),
            None => error,
        });
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

#[cfg(test)]
mod tests {
    use super::admit_toml_parser;
    use crate::ConfigError;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn test_guard_span_reports_structured_context_location() {
        let mut session = SourceLoadSession::new("TOML:test", SourceLimits::unbounded());
        let error = admit_toml_parser("TOML:test", "value = [1", &mut session)
            .expect_err("the parser must reject an unterminated TOML array");
        assert!(matches!(error, ConfigError::SourceParseError { .. }));
        assert_eq!(
            error
                .source_location()
                .map(|location| (location.line(), location.column())),
            Some((1, Some(9)))
        );
    }
}
