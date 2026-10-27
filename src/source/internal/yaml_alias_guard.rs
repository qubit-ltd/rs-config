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

/// Rejects YAML anchors and aliases before the value backend builds an AST.
///
/// Scanner errors and unsupported tokens produce fixed, source-aware
/// diagnostics. Parser-owned text and token payloads are never included in an
/// error.
pub(in crate::source) fn reject_yaml_aliases(label: &str, content: &str) -> ConfigResult<()> {
    let mut scanner = Scanner::new(content.chars());

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
        if matches!(token_type, TokenType::Anchor(_) | TokenType::Alias(_)) {
            return Err(ConfigError::source_parse_error(
                label,
                format!(
                    "YAML anchors and aliases are not supported in '{label}' at line {}, column {}",
                    marker.line(),
                    marker.col() + 1,
                ),
            ));
        }
    }

    if let Some(error) = scanner.get_error() {
        let marker = error.marker();
        return Err(ConfigError::source_parse_error(
            label,
            format!(
                "Failed to parse YAML file '{label}' at line {}, column {}: invalid YAML syntax",
                marker.line(),
                marker.col() + 1,
            ),
        ));
    }

    Ok(())
}
