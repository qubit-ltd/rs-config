// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Conservative depth accounting for TOML parser events.

use qubit_budget::BudgetError;
use qubit_budget::LimitExceededError;
use toml_parser::ErrorSink;
use toml_parser::Span;
use toml_parser::decoder::Encoding;
use toml_parser::parser::EventReceiver;

use super::super::source_load_session::SourceLoadSession;
use crate::ConfigError;
use crate::ConfigResult;

#[derive(Clone, Copy)]
enum Frame {
    Array(usize),
    Inline(usize),
}

/// Tracks bounded TOML nesting without constructing parser values.
pub(super) struct TomlParserDepth<'a, 'b> {
    session: &'a SourceLoadSession<'b>,
    table_depth: usize,
    key_segments: usize,
    value_depth: usize,
    frames: Vec<Frame>,
    in_header: bool,
    first_error: Option<ConfigError>,
}

impl<'a, 'b> TomlParserDepth<'a, 'b> {
    pub(super) fn new(session: &'a SourceLoadSession<'b>) -> Self {
        Self {
            session,
            table_depth: 0,
            key_segments: 0,
            value_depth: 0,
            frames: Vec::with_capacity(8),
            in_header: false,
            first_error: None,
        }
    }

    pub(super) fn finish(self) -> ConfigResult<()> {
        match self.first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn check(&mut self, depth: usize) -> bool {
        if self.first_error.is_some() {
            return false;
        }
        if let Err(error) = self.session.check_parser_depth(depth) {
            self.first_error = Some(error);
            false
        } else {
            true
        }
    }

    fn reject_unrepresentable_depth(&mut self) {
        if self.first_error.is_some() {
            return;
        }
        if let Err(error) = self.session.check_parser_depth(usize::MAX) {
            self.first_error = Some(error);
            return;
        }
        // The actual observation is greater than usize::MAX and therefore
        // cannot be represented. Retain its saturated lower bound.
        let source_id = self.session.source_id().to_owned();
        let source = BudgetError::from(LimitExceededError::at_least(
            super::super::source_limit_kind::SourceLimitKind::ParserNestingDepth,
            usize::MAX,
            usize::MAX,
        ));
        self.first_error = Some(ConfigError::SourceLimitExceeded {
            source_id: source_id.clone(),
            budget_id: source_id,
            kind: super::super::source_limit_kind::SourceLimitKind::ParserNestingDepth,
            limit: usize::MAX,
            observed_at_least: usize::MAX,
            source,
        });
    }

    fn add_depth(left: usize, right: usize) -> Option<usize> {
        left.checked_add(right)
    }

    fn key_depth(&self) -> Option<usize> {
        self.key_segments.checked_mul(2)
    }
}

impl EventReceiver for TomlParserDepth<'_, '_> {
    fn std_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if self.first_error.is_none() {
            self.in_header = true;
            self.key_segments = 0;
        }
    }

    fn std_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        let Some(depth) = self.key_depth() else {
            self.reject_unrepresentable_depth();
            return;
        };
        if self.check(depth) {
            self.table_depth = depth;
        }
        self.in_header = false;
        self.key_segments = 0;
        self.value_depth = self.table_depth;
    }

    fn array_table_open(&mut self, span: Span, error: &mut dyn ErrorSink) {
        self.std_table_open(span, error);
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        let Some(depth) = self.key_depth() else {
            self.reject_unrepresentable_depth();
            return;
        };
        if self.check(depth) {
            self.table_depth = depth;
        }
        self.in_header = false;
        self.key_segments = 0;
        self.value_depth = self.table_depth;
    }

    fn simple_key(&mut self, _span: Span, _kind: Option<Encoding>, _error: &mut dyn ErrorSink) {
        if self.first_error.is_some() {
            return;
        }
        let Some(segments) = self.key_segments.checked_add(1) else {
            self.reject_unrepresentable_depth();
            return;
        };
        self.key_segments = segments;
    }

    fn key_val_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if self.in_header || self.first_error.is_some() {
            return;
        }
        let base_depth = match self.frames.last() {
            Some(Frame::Inline(parent)) => parent.checked_add(1),
            _ => Some(self.table_depth),
        };
        let Some(depth) = base_depth.and_then(|base| self.key_depth().and_then(|key| Self::add_depth(base, key)))
        else {
            self.reject_unrepresentable_depth();
            return;
        };
        if self.check(depth) {
            self.value_depth = depth;
        }
        self.key_segments = 0;
    }

    fn array_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        let Some(depth) = self.value_depth.checked_add(1) else {
            self.reject_unrepresentable_depth();
            return false;
        };
        if !self.check(depth) {
            return false;
        }
        self.frames.push(Frame::Array(self.value_depth));
        self.value_depth = depth;
        true
    }

    fn array_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if let Some(Frame::Array(parent)) = self.frames.pop() {
            self.value_depth = parent;
        }
    }

    fn inline_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        let Some(depth) = self.value_depth.checked_add(1) else {
            self.reject_unrepresentable_depth();
            return false;
        };
        if !self.check(depth) {
            return false;
        }
        self.frames.push(Frame::Inline(self.value_depth));
        self.value_depth = depth;
        true
    }

    fn inline_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        if let Some(Frame::Inline(parent)) = self.frames.pop() {
            self.value_depth = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use toml_parser::Span;
    use toml_parser::parser::EventReceiver;

    use super::Frame;
    use super::TomlParserDepth;
    use crate::ConfigError;
    use crate::source::SourceLimitKind;
    use crate::source::SourceLimits;
    use crate::source::source_load_session::SourceLoadSession;

    #[test]
    fn unrepresentable_key_depth_fails_closed_and_stops_state_growth() {
        let session = SourceLoadSession::new("TOML:test", SourceLimits::unbounded());
        let mut receiver = TomlParserDepth::new(&session);
        receiver.key_segments = usize::MAX;
        receiver.std_table_close(Span::new_unchecked(0, 0), &mut ());
        receiver.simple_key(Span::new_unchecked(0, 0), None, &mut ());

        assert!(matches!(
            receiver.first_error,
            Some(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserNestingDepth,
                limit: usize::MAX,
                observed_at_least: usize::MAX,
                ..
            })
        ));
        assert_eq!(receiver.key_segments, usize::MAX);
    }

    #[test]
    fn parser_depth_rejection_is_retained_and_stops_later_checks() {
        let limits = SourceLimits::builder().max_parser_nesting_depth(0).build();
        let session = SourceLoadSession::new("TOML:test", limits);
        let mut receiver = TomlParserDepth::new(&session);

        assert!(!receiver.array_open(Span::new_unchecked(0, 0), &mut ()));
        assert!(!receiver.inline_table_open(Span::new_unchecked(0, 0), &mut ()));
        let error = receiver
            .finish()
            .expect_err("the first over-depth event must be preserved");
        assert!(matches!(
            error,
            ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserNestingDepth,
                observed_at_least: 1,
                ..
            }
        ));
    }

    #[test]
    fn arithmetic_overflow_fails_closed_for_key_and_value_depths() {
        let session = SourceLoadSession::new("TOML:test", SourceLimits::unbounded());
        let mut key_receiver = TomlParserDepth::new(&session);
        key_receiver.frames.push(Frame::Inline(usize::MAX));
        key_receiver.key_val_sep(Span::new_unchecked(0, 0), &mut ());
        assert!(matches!(
            key_receiver.first_error,
            Some(ConfigError::SourceLimitExceeded {
                kind: SourceLimitKind::ParserNestingDepth,
                ..
            })
        ));

        let mut array_receiver = TomlParserDepth::new(&session);
        array_receiver.value_depth = usize::MAX;
        assert!(!array_receiver.array_open(Span::new_unchecked(0, 0), &mut ()));

        let mut inline_receiver = TomlParserDepth::new(&session);
        inline_receiver.value_depth = usize::MAX;
        assert!(!inline_receiver.inline_table_open(Span::new_unchecked(0, 0), &mut ()));
    }
}
