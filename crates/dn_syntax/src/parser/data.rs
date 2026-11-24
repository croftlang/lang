//! Values, entities, aggregates, enums, identities, constants, errors and events.

use super::{ParseResult, Parser};
use crate::ast::{ConstDecl, EnumDecl, IdentityDecl, RecordDecl, ValueDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// A value or an entity: fields and invariants in braces.
    pub(super) fn parse_value(&mut self, keyword: Keyword) -> ParseResult<ValueDecl> {
        let doc = self.doc();
        self.expect_keyword(keyword)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let (mut fields, mut invariants) = (Vec::new(), Vec::new());
        while !self.eat(&TokenKind::RightBrace) {
            if self.check_keyword(Keyword::Invariant) {
                invariants.push(self.parse_invariant()?);
            } else {
                fields.push(self.parse_field()?);
                self.eat(&TokenKind::Comma);
            }
        }
        Ok(ValueDecl {
            doc,
            name,
            fields,
            invariants,
        })
    }

    pub(super) fn parse_enum(&mut self) -> ParseResult<EnumDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Enum)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let (mut includes, mut variants) = (Vec::new(), Vec::new());
        while !self.eat(&TokenKind::RightBrace) {
            if self.check_word("include") {
                self.advance();
                includes.push(self.parse_path()?);
                continue;
            }
            let doc = self.doc();
            let name = self.expect_identifier()?;
            let fields = if self.check(&TokenKind::LeftParen) {
                self.parse_variant_fields()?
            } else {
                Vec::new()
            };
            variants.push(RecordDecl { doc, name, fields });
            self.eat(&TokenKind::Comma);
        }
        Ok(EnumDecl {
            doc,
            name,
            includes,
            variants,
        })
    }

    pub(super) fn parse_const(&mut self) -> ParseResult<ConstDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Const)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Equal)?;
        let value = self.parse_expression()?;
        Ok(ConstDecl {
            doc,
            name,
            ty,
            value,
        })
    }

    pub(super) fn parse_identity(&mut self) -> ParseResult<IdentityDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Identity)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let rules = self.parse_rules()?;
        Ok(IdentityDecl {
            doc,
            name,
            ty,
            rules,
        })
    }
}
