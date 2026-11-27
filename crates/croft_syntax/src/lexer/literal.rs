//! Strings, numbers and words.

use super::Lexer;
use crate::keyword::Keyword;
use crate::span::{Diagnostic, Span};
use crate::token::TokenKind;

impl<'a> Lexer<'a> {
    /// Reads a string after its opening `"`, turning escapes such as `\n`
    /// into the characters they stand for.
    pub(crate) fn read_string(&mut self, start: Span) -> Result<TokenKind, Diagnostic> {
        let mut value = String::new();

        while let Some(c) = self.advance() {
            match c {
                '"' => return Ok(TokenKind::String(value)),
                '\\' => match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    other => {
                        let found = other.map(String::from).unwrap_or_default();
                        let message = format!("unknown escape `\\{found}`");
                        return Err(Diagnostic::new(message, self.current_span()));
                    }
                },
                c => value.push(c),
            }
        }

        Err(Diagnostic::new("unterminated string", start))
    }

    /// Reads a number: decimal, hex after `0x` or binary after `0b`, with `_`
    /// separators allowed, or a `Float` when a `.` and digits follow.
    pub(crate) fn read_number(
        &mut self,
        first: char,
        start: Span,
    ) -> Result<TokenKind, Diagnostic> {
        let radix = match (first, self.peek()) {
            ('0', Some('x')) => Some((16, "hex", "0x")),
            ('0', Some('b')) => Some((2, "binary", "0b")),
            _ => None,
        };

        if let Some((radix, name, prefix)) = radix {
            self.advance();

            let digits = self.read_digits(|c| c.is_digit(radix));

            return i64::from_str_radix(&digits, radix)
                .map(TokenKind::Int)
                .map_err(|_| {
                    Diagnostic::new(format!("invalid {name} number `{prefix}{digits}`"), start)
                });
        }

        let mut text = first.to_string() + &self.read_digits(|c| c.is_ascii_digit());
        let is_float =
            self.peek() == Some('.') && self.peek_second().is_some_and(|c| c.is_ascii_digit());

        if is_float {
            self.advance();
            text.push('.');
            text += &self.read_digits(|c| c.is_ascii_digit());
            return Ok(TokenKind::Float(text));
        }

        text.parse()
            .map(TokenKind::Int)
            .map_err(|_| Diagnostic::new(format!("number `{text}` is too large"), start))
    }

    /// Reads digits accepted by `is_digit`, skipping `_` separators.
    fn read_digits(&mut self, is_digit: impl Fn(char) -> bool) -> String {
        let mut digits = String::new();

        while let Some(c) = self.peek() {
            if c == '_' {
                self.advance();
            } else if is_digit(c) {
                digits.push(c);
                self.advance();
            } else {
                break;
            }
        }

        digits
    }

    /// Reads a name, or a keyword when the word is reserved.
    pub(crate) fn read_word(&mut self, first: char) -> TokenKind {
        let mut word = first.to_string();

        while let Some(c) = self.peek().filter(|c| c.is_alphanumeric() || *c == '_') {
            word.push(c);
            self.advance();
        }

        match Keyword::from_word(&word) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None => TokenKind::Identifier(word),
        }
    }
}
