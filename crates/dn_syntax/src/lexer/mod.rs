//! Turns source text into tokens.

mod comment;
mod literal;
mod symbol;
#[cfg(test)]
mod tests;

use crate::span::{Diagnostic, SourceId, Span};
use crate::token::{Token, TokenKind};

pub fn tokenize(source: &str, source_id: SourceId) -> Result<Vec<Token>, Diagnostic> {
    Lexer::new(source, source_id).tokenize_all()
}

pub(crate) struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    source_id: SourceId,
    line: u32,
    column: u32,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, source_id: SourceId) -> Self {
        let chars = source.chars().peekable();
        Self {
            chars,
            source_id,
            line: 1,
            column: 1,
        }
    }

    pub(crate) fn current_span(&self) -> Span {
        Span {
            source: self.source_id,
            line: self.line,
            column: self.column,
        }
    }

    pub(crate) fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    pub(crate) fn peek_second(&self) -> Option<char> {
        let mut lookahead = self.chars.clone();
        lookahead.next();
        lookahead.next()
    }

    pub(crate) fn advance(&mut self) -> Option<char> {
        let next = self.chars.next()?;
        if next == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(next)
    }

    fn tokenize_all(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            let start_line = self.line;
            let doc = self.skip_whitespace_and_comments();
            let newline_before = self.line > start_line || tokens.is_empty();
            let span = self.current_span();
            let Some(current) = self.advance() else {
                tokens.push(Token {
                    kind: TokenKind::EndOfFile,
                    span,
                    newline_before,
                    doc,
                });
                return Ok(tokens);
            };
            let kind = self.read_token(current, span)?;
            tokens.push(Token {
                kind,
                span,
                newline_before,
                doc,
            });
        }
    }
}
