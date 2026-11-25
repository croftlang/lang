//! Punctuation and operators.

use super::Lexer;
use crate::span::{Diagnostic, Span};
use crate::token::TokenKind;

impl Lexer<'_> {
    fn advance_if(&mut self, expected: char) -> bool {
        let matches = self.peek() == Some(expected);

        if matches {
            self.advance();
        }

        matches
    }

    pub(super) fn read_token(
        &mut self,
        current: char,
        span: Span,
    ) -> Result<TokenKind, Diagnostic> {
        let kind = match current {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '?' => TokenKind::Question,
            '-' if self.advance_if('>') => TokenKind::Arrow,
            '-' => TokenKind::Minus,
            '<' if self.advance_if('=') => TokenKind::LessEqual,
            '<' if self.advance_if('<') => TokenKind::ShiftLeft,
            '<' => TokenKind::Less,
            '>' if self.advance_if('=') => TokenKind::GreaterEqual,
            '>' if self.advance_if('>') => TokenKind::ShiftRight,
            '>' => TokenKind::Greater,
            '=' if self.advance_if('=') => TokenKind::EqualEqual,
            '=' => TokenKind::Equal,
            '!' if self.advance_if('=') => TokenKind::NotEqual,
            '!' => TokenKind::Not,
            '&' if self.advance_if('&') => TokenKind::And,
            '&' => TokenKind::Ampersand,
            '|' if self.advance_if('|') => TokenKind::Or,
            '|' => TokenKind::Pipe,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '"' => self.read_string(span)?,
            c if c.is_ascii_digit() => self.read_number(c, span)?,
            c if c.is_alphabetic() || c == '_' => self.read_word(c),
            other => {
                return Err(Diagnostic::new(
                    format!("unexpected character `{other}`"),
                    span,
                ))
            }
        };

        Ok(kind)
    }
}
