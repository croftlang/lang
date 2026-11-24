//! `domain Game { contexts { ... } use { ... } from X  uses Keyboard } = { ... }`

use super::{ParseResult, Parser};
use crate::ast::DomainDecl;
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_domain(&mut self) -> ParseResult<DomainDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Domain)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let (mut contexts, mut uses_from, mut uses) = (Vec::new(), Vec::new(), Vec::new());
        while !self.eat(&TokenKind::RightBrace) {
            if self.check_word("contexts") {
                self.advance();
                let paths = self.parse_path_list()?;
                contexts.extend(paths.into_iter().map(|path| path.last().clone()));
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
            uses_from,
            uses,
            body,
        })
    }
}
