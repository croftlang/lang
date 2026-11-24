//! `domain Game { contexts { Dungeon -> Scoring } use { ... } from X  uses Keyboard } = { ... }`

use super::{ParseResult, Parser};
use crate::ast::{DomainDecl, Identifier};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_domain(&mut self) -> ParseResult<DomainDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Domain)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let (mut contexts, mut relations) = (Vec::new(), Vec::new());
        let (mut uses_from, mut uses) = (Vec::new(), Vec::new());
        while !self.eat(&TokenKind::RightBrace) {
            if self.check_word("contexts") {
                self.advance();
                self.parse_context_map(&mut contexts, &mut relations)?;
            } else if self.check_keyword(Keyword::Use) {
                uses_from.push(self.parse_use()?);
            } else if self.eat_keyword(Keyword::Uses) {
                loop {
                    uses.push(self.parse_path()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
            } else {
                return Err(self.unexpected("`contexts`, `use` or `uses`"));
            }
        }
        self.expect(&TokenKind::Equal)?;
        let body = self.parse_expression()?;
        Ok(DomainDecl {
            doc,
            name,
            contexts,
            relations,
            uses_from,
            uses,
            body,
        })
    }

    /// `{ Dungeon -> Scoring  Dungeon -> Platform }`: each arrow goes from an
    /// upstream context to one downstream of it.
    fn parse_context_map(
        &mut self,
        contexts: &mut Vec<Identifier>,
        relations: &mut Vec<(Identifier, Identifier)>,
    ) -> ParseResult<()> {
        self.expect(&TokenKind::LeftBrace)?;
        while !self.eat(&TokenKind::RightBrace) {
            let mut upstream = self.expect_identifier()?;
            contexts.push(upstream.clone());
            while self.eat(&TokenKind::Arrow) {
                let downstream = self.expect_identifier()?;
                contexts.push(downstream.clone());
                relations.push((upstream, downstream.clone()));
                upstream = downstream;
            }
            self.eat(&TokenKind::Comma);
        }
        Ok(())
    }
}
