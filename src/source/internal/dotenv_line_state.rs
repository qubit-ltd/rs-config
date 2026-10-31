// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tracks dotenv logical-line boundaries independently from value parsing.

/// The state used by dotenvy's logical-line scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::source) enum DotenvLineState {
    /// No quoted value or escape is currently open.
    Complete,
    /// A backslash outside quotes escapes the next character.
    Escape,
    /// A single-quoted value is open.
    StrongOpen,
    /// A backslash inside a single-quoted value escapes the next character
    /// for logical-line boundary detection.
    StrongOpenEscape,
    /// A double-quoted value is open.
    WeakOpen,
    /// A backslash inside a double-quoted value escapes the next character.
    WeakOpenEscape,
    /// The rest of the physical line is a comment.
    Comment,
    /// Whitespace follows a complete token and may introduce a comment.
    WhiteSpace,
}

impl DotenvLineState {
    /// Advances the logical-line scanner by one character.
    pub(in crate::source) fn advance(&mut self, character: char) {
        *self = match *self {
            Self::Complete => match character {
                value if value.is_whitespace() && !matches!(value, '\r' | '\n') => Self::WhiteSpace,
                '\\' => Self::Escape,
                '\'' => Self::StrongOpen,
                '"' => Self::WeakOpen,
                _ => Self::Complete,
            },
            Self::WhiteSpace => match character {
                '#' => Self::Comment,
                '\\' => Self::Escape,
                '\'' => Self::StrongOpen,
                '"' => Self::WeakOpen,
                _ => Self::Complete,
            },
            Self::Escape => Self::Complete,
            Self::StrongOpen => match character {
                '\\' => Self::StrongOpenEscape,
                '\'' => Self::Complete,
                _ => Self::StrongOpen,
            },
            Self::StrongOpenEscape => Self::StrongOpen,
            Self::WeakOpen => match character {
                '\\' => Self::WeakOpenEscape,
                '"' => Self::Complete,
                _ => Self::WeakOpen,
            },
            Self::WeakOpenEscape => Self::WeakOpen,
            Self::Comment => Self::Comment,
        };

        if character == '\n' && *self == Self::Comment {
            *self = Self::Complete;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DotenvLineState;

    #[test]
    fn advances_through_each_logical_line_state() {
        let cases = [
            (DotenvLineState::Complete, ' ', DotenvLineState::WhiteSpace),
            (DotenvLineState::Complete, '\r', DotenvLineState::Complete),
            (DotenvLineState::Complete, '\n', DotenvLineState::Complete),
            (DotenvLineState::Complete, '\\', DotenvLineState::Escape),
            (DotenvLineState::Complete, '\'', DotenvLineState::StrongOpen),
            (DotenvLineState::Complete, '"', DotenvLineState::WeakOpen),
            (DotenvLineState::WhiteSpace, '#', DotenvLineState::Comment),
            (DotenvLineState::WhiteSpace, 'x', DotenvLineState::Complete),
            (DotenvLineState::Escape, 'x', DotenvLineState::Complete),
            (DotenvLineState::StrongOpen, '\\', DotenvLineState::StrongOpenEscape),
            (DotenvLineState::StrongOpen, '\'', DotenvLineState::Complete),
            (DotenvLineState::StrongOpen, 'x', DotenvLineState::StrongOpen),
            (DotenvLineState::StrongOpenEscape, 'x', DotenvLineState::StrongOpen),
            (DotenvLineState::WeakOpen, '\\', DotenvLineState::WeakOpenEscape),
            (DotenvLineState::WeakOpen, '"', DotenvLineState::Complete),
            (DotenvLineState::WeakOpen, 'x', DotenvLineState::WeakOpen),
            (DotenvLineState::WeakOpenEscape, 'x', DotenvLineState::WeakOpen),
            (DotenvLineState::Comment, 'x', DotenvLineState::Comment),
        ];

        for (mut state, character, expected) in cases {
            state.advance(character);
            assert_eq!(state, expected, "state {state:?} after {character:?}");
        }

        let mut comment = DotenvLineState::Comment;
        comment.advance('\n');
        assert_eq!(comment, DotenvLineState::Complete);
    }
}
