//! Files and contexts.
//!
//! A context is written inline, `context Dungeon { ... }`, or as a header line,
//! `context Dungeon`, which claims the rest of the file.

use super::{ParseResult, Parser};
use crate::ast::{ContextDecl, ContextItem, File, UseDecl};
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_file(&mut self) -> ParseResult<File> {
        let mut file = File::default();
        while !self.check(&TokenKind::EndOfFile) {
            if self.check_keyword(Keyword::Domain) {
                file.domains.push(self.parse_domain()?);
                continue;
            }
            if !self.check_keyword(Keyword::Context) {
                return Err(self.unexpected("`context` or `domain`"));
            }
            file.contexts.push(self.parse_context()?);
        }
        Ok(file)
    }

    fn parse_context(&mut self) -> ParseResult<ContextDecl> {
        self.expect_keyword(Keyword::Context)?;
        let name = self.expect_identifier()?;
        let braced = self.eat(&TokenKind::LeftBrace);
        let end = if braced {
            TokenKind::RightBrace
        } else {
            TokenKind::EndOfFile
        };
        let mut items = Vec::new();
        while !self.check(&end) {
            if self.check(&TokenKind::EndOfFile) {
                return Err(self.unexpected("`}`"));
            }
            items.push(self.parse_context_item(braced)?);
        }
        if braced {
            self.advance();
        }
        Ok(ContextDecl { name, items })
    }

    fn parse_context_item(&mut self, braced: bool) -> ParseResult<ContextItem> {
        let TokenKind::Keyword(keyword) = self.peek().kind else {
            return Err(self.unexpected("a declaration"));
        };
        let item = match keyword {
            Keyword::Publish => {
                self.advance();
                ContextItem::Publish(self.parse_path_list()?)
            }
            Keyword::Use => ContextItem::Use(self.parse_use()?),
            Keyword::Value => ContextItem::Value(self.parse_value(Keyword::Value)?),
            Keyword::Enum => ContextItem::Enum(self.parse_enum()?),
            Keyword::Error => ContextItem::Error(self.parse_record(Keyword::Error)?),
            Keyword::Const => ContextItem::Const(self.parse_const()?),
            Keyword::Identity => ContextItem::Identity(self.parse_identity()?),
            Keyword::Constraint => ContextItem::Constraint(self.parse_constraint()?),
            Keyword::Aggregate => ContextItem::Aggregate(self.parse_aggregate(braced)?),
            Keyword::Fn | Keyword::Service | Keyword::Translation => {
                ContextItem::Function(self.parse_function()?)
            }
            Keyword::Policy => ContextItem::Policy(self.parse_policy()?),
            Keyword::Port => ContextItem::Port(self.parse_port()?),
            Keyword::Adapter => ContextItem::Adapter(self.parse_adapter()?),
            Keyword::Repository => ContextItem::Repository(self.parse_repository()?),
            Keyword::Entity => {
                let message = "an entity belongs inside an aggregate; a thing with identity \
                               on its own is an `aggregate` with no entities";
                return Err(Diagnostic::new(message, self.span()));
            }
            _ => return Err(self.unexpected("a declaration")),
        };
        Ok(item)
    }

    /// `use { EnemyId, Room.EnemyDefeated } from Dungeon`
    pub(super) fn parse_use(&mut self) -> ParseResult<UseDecl> {
        self.expect_keyword(Keyword::Use)?;
        let names = self.parse_path_list()?;
        self.expect_word("from")?;
        let from = self.parse_path()?;
        Ok(UseDecl { names, from })
    }
}
