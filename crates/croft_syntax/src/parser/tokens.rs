//! Reading and checking the next token.

use super::{ParseResult, Parser};
use crate::ast::Identifier;
use crate::keyword::Keyword;
use crate::span::{Diagnostic, Span};
use crate::token::{Token, TokenKind};

impl Parser {
    /// The next token, without reading it.
    pub(super) fn peek(&self) -> &Token {
        self.peek_nth(0)
    }

    /// The token `offset` places ahead, or `EndOfFile` past the end.
    pub(super) fn peek_nth(&self, offset: usize) -> &Token {
        let index = (self.position + offset).min(self.tokens.len() - 1);
        &self.tokens[index]
    }

    /// Where the next token is.
    pub(super) fn span(&self) -> Span {
        self.peek().span
    }

    /// Reads the next token. It stays at `EndOfFile` once there.
    pub(super) fn advance(&mut self) -> Token {
        let token = self.peek().clone();

        if token.kind != TokenKind::EndOfFile {
            self.position += 1;
        }

        token
    }

    /// Whether the next token is of `kind`.
    pub(super) fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    /// Whether the next token is `keyword`.
    pub(super) fn check_keyword(&self, keyword: Keyword) -> bool {
        self.check(&TokenKind::Keyword(keyword))
    }

    /// Whether the next token is `word`, a name that is only special in one
    /// place, such as `from` or `for`.
    pub(super) fn check_word(&self, word: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(name) if name == word)
    }

    /// Reads the next token if it is of `kind`, and says whether it did.
    pub(super) fn eat(&mut self, kind: &TokenKind) -> bool {
        let matches = self.check(kind);

        if matches {
            self.advance();
        }

        matches
    }

    /// Reads the next token if it is `keyword`, and says whether it did.
    pub(super) fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        self.eat(&TokenKind::Keyword(keyword))
    }

    /// Reads the next token, which must be of `kind`.
    pub(super) fn expect(&mut self, kind: &TokenKind) -> ParseResult<Token> {
        if self.check(kind) {
            return Ok(self.advance());
        }

        Err(self.unexpected(&kind.describe()))
    }

    /// Reads the next token, which must be `keyword`.
    pub(super) fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<Token> {
        self.expect(&TokenKind::Keyword(keyword))
    }

    /// Reads the next token, which must be the name `word`.
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

    /// A field's name after `.` or in a record, where a keyword is a name
    /// too: `score.value`, `Score { value: 3 }`.
    pub(super) fn expect_field_name(&mut self) -> ParseResult<Identifier> {
        if let TokenKind::Keyword(keyword) = self.peek().kind {
            let span = self.span();

            self.advance();

            return Ok(Identifier {
                name: keyword.as_str().to_string(),
                span,
            });
        }

        self.expect_identifier()
    }

    /// Whether the next token can be read as a name.
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

    /// An error saying what was expected and what the next token is instead.
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
