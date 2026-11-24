//! Ports, adapters and repositories: where the domain meets the outside world.

use super::{ParseResult, Parser};
use crate::ast::{AdapterDecl, FunctionDecl, FunctionKind, PortDecl, RepositoryDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// `port Screen { fn clear(): Unit }`
    pub(super) fn parse_port(&mut self) -> ParseResult<PortDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Port)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let mut operations = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            operations.push(self.parse_operation()?);
        }
        Ok(PortDecl {
            doc,
            name,
            operations,
        })
    }

    /// `adapter SpectrumScreen for Screen on target spectrum_48k { ... }`
    pub(super) fn parse_adapter(&mut self) -> ParseResult<AdapterDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Adapter)?;
        let name = self.expect_identifier()?;
        self.expect_word("for")?;
        let port = self.parse_path()?;
        self.expect_word("on")?;
        self.expect_word("target")?;
        let target = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let (mut fields, mut operations) = (Vec::new(), Vec::new());
        while !self.eat(&TokenKind::RightBrace) {
            if self.check_keyword(Keyword::Fn) {
                operations.push(self.parse_operation()?);
            } else {
                fields.push(self.parse_field()?);
            }
        }
        Ok(AdapterDecl {
            doc,
            name,
            port,
            target,
            fields,
            operations,
        })
    }

    /// An operation is written with `fn`, but belongs to a port or adapter.
    fn parse_operation(&mut self) -> ParseResult<FunctionDecl> {
        if !self.check_keyword(Keyword::Fn) {
            return Err(self.unexpected("`fn`"));
        }
        let mut operation = self.parse_function()?;
        operation.kind = FunctionKind::Operation;
        Ok(operation)
    }

    /// `repository Rooms for Room in Storage`
    pub(super) fn parse_repository(&mut self) -> ParseResult<RepositoryDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Repository)?;
        let name = self.expect_identifier()?;
        self.expect_word("for")?;
        let aggregate = self.parse_path()?;
        let storage = if self.check_word("in") && !self.peek().newline_before {
            self.advance();
            Some(self.parse_path()?)
        } else {
            None
        };
        Ok(RepositoryDecl {
            doc,
            name,
            aggregate,
            storage,
        })
    }
}
