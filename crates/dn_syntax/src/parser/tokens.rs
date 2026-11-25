//! Reading and checking the next token.

use super::{ParseResult, Parser};
use crate::ast::Identifier;
use crate::keyword::Keyword;
use crate::span::{Diagnostic, Span};
use crate::token::{Token, TokenKind};

impl Parser {
    pub(super) fn peek(&self) -> &Token {
        self.peek_nth(0)
    }

    pub(super) fn peek_nth(&self, offset: usize) -> &Token {
        let index = (self.position + offset).min(self.tokens.len() - 1);
        &self.tokens[index]
    }

    pub(super) fn span(&self) -> Span {
        self.peek().span
    }

    pub(super) fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if token.kind != TokenKind::EndOfFile {
            self.position += 1;
        }
        token
    }

    pub(super) fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    pub(super) fn check_keyword(&self, keyword: Keyword) -> bool {
        self.check(&TokenKind::Keyword(keyword))
    }

    /// Is the next token the given contextual word, such as `from` or `target`?
    pub(super) fn check_word(&self, word: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(name) if name == word)
    }

    pub(super) fn eat(&mut self, kind: &TokenKind) -> bool {
        let matches = self.check(kind);
        if matches {
            self.advance();
        }
        matches
    }

    pub(super) fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        self.eat(&TokenKind::Keyword(keyword))
    }

    pub(super) fn expect(&mut self, kind: &TokenKind) -> ParseResult<Token> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        Err(self.unexpected(&kind.describe()))
    }

    pub(super) fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<Token> {
        self.expect(&TokenKind::Keyword(keyword))
    }

    pub(super) fn expect_word(&mut self, word: &str) -> ParseResult<()> {
        if self.check_word(word) {
            self.advance();
            return Ok(());
        }
        Err(self.unexpected(&format!("`{word}`")))
    }

    /// A name. `event` and `error` also work as names, as in `event.source`.
    pub(super) fn expect_identifier(&mut self) -> ParseResult<Identifier> {
        let span = self.span();
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            TokenKind::Keyword(Keyword::Event) => "event".to_string(),
            TokenKind::Keyword(Keyword::Error) => "error".to_string(),
            _ => return Err(self.unexpected("a name")),
        };
        self.advance();
        Ok(Identifier { name, span })
    }

    pub(super) fn is_identifier(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Identifier(_) | TokenKind::Keyword(Keyword::Event | Keyword::Error)
        )
    }

    /// The doc comment written directly above the next token.
    pub(super) fn doc(&self) -> Vec<String> {
        self.peek().doc.clone()
    }

    pub(super) fn unexpected(&self, expected: &str) -> Diagnostic {
        let found = self.peek().kind.describe();
        Diagnostic::new(format!("expected {expected}, found {found}"), self.span())
    }

    /// Turns the `>>` ahead into `>`, as if its first `>` had been read.
    pub(super) fn split_shift_right(&mut self) {
        let index = self.position.min(self.tokens.len() - 1);
        self.tokens[index].kind = TokenKind::Greater;
    }
}
