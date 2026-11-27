//! `domain Game { contexts { Dungeon -> Scoring } import { ... } from X  calls { Keyboard } } = { ... }`

use super::{ParseResult, Parser};
use crate::ast::{DomainDecl, Identifier};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// The domain: its context map, imports and called ports in braces, then
    /// its body after `=`.
    pub(super) fn parse_domain(&mut self) -> ParseResult<DomainDecl> {
        let doc = self.doc();

        self.expect_keyword(Keyword::Domain)?;

        let name = self.expect_identifier()?;

        self.expect(&TokenKind::LeftBrace)?;

        let (mut contexts, mut relations) = (Vec::new(), Vec::new());
        let (mut imports, mut calls) = (Vec::new(), Vec::new());

        while !self.eat(&TokenKind::RightBrace) {
            self.reject_uses()?;
            self.reject_old_term_list()?;

            if self.check_word("contexts") {
                self.advance();
                self.parse_context_map(&mut contexts, &mut relations)?;
            } else if self.check_keyword(Keyword::Import) {
                imports.push(self.parse_import()?);
            } else if self.check_keyword(Keyword::Calls) {
                calls.extend(self.parse_calls()?);
            } else {
                return Err(self.unexpected("`contexts`, `import` or `calls`"));
            }
        }

        self.expect(&TokenKind::Equal)?;

        let body = self.parse_expression()?;

        Ok(DomainDecl {
            doc,
            name,
            contexts,
            relations,
            imports,
            calls,
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
