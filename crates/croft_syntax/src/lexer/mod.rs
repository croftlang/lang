//! Turns source text into tokens.

mod comment;
mod literal;
mod symbol;
#[cfg(test)]
mod tests;

use std::ops::Range;

use crate::span::{Diagnostic, SourceId, Span};
use crate::token::{Comment, Token, TokenKind};

pub fn tokenize(source: &str, source_id: SourceId) -> Result<Vec<Token>, Diagnostic> {
    tokenize_with_comments(source, source_id).map(|tokenized| tokenized.tokens)
}

/// Everything the lexer reads from a file, as the formatter needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenizedSource {
    pub tokens: Vec<Token>,
    /// Where each token is in the source, in bytes, one for each token.
    pub ranges: Vec<Range<usize>>,
    /// Every comment, in the order written.
    pub comments: Vec<Comment>,
}

/// Reads the tokens, where each one was written, and every comment.
pub fn tokenize_with_comments(
    source: &str,
    source_id: SourceId,
) -> Result<TokenizedSource, Diagnostic> {
    Lexer::new(source, source_id).tokenize_all()
}

pub(crate) struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    source_id: SourceId,
    line: u32,
    column: u32,
    /// How many bytes have been read.
    offset: usize,
    /// Every comment read so far.
    comments: Vec<Comment>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, source_id: SourceId) -> Self {
        let chars = source.chars().peekable();

        Self {
            chars,
            source_id,
            line: 1,
            column: 1,
            offset: 0,
            comments: Vec::new(),
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

        self.offset += next.len_utf8();

        if next == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(next)
    }

    fn tokenize_all(mut self) -> Result<TokenizedSource, Diagnostic> {
        let mut tokens = Vec::new();
        let mut ranges = Vec::new();

        loop {
            let start_line = self.line;
            let doc = self.skip_whitespace_and_comments();
            let newline_before = self.line > start_line || tokens.is_empty();
            let span = self.current_span();
            let start = self.offset;

            let Some(current) = self.advance() else {
                tokens.push(Token {
                    kind: TokenKind::EndOfFile,
                    span,
                    newline_before,
                    doc,
                });

                ranges.push(start..start);

                return Ok(TokenizedSource {
                    tokens,
                    ranges,
                    comments: self.comments,
                });
            };

            let kind = self.read_token(current, span)?;

            tokens.push(Token {
                kind,
                span,
                newline_before,
                doc,
            });

            ranges.push(start..self.offset);
        }
    }
}
