//! Loops, which run until `break`, or once for each value a counter allows.

use super::{ParseResult, Parser};
use crate::ast::{Expr, ExprKind, LoopCounter};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// `loop { ... }` or `loop (let i where Range(0, 99)) { ... }`
    pub(super) fn parse_loop(&mut self) -> ParseResult<Expr> {
        let span = self.span();
        self.expect_keyword(Keyword::Loop)?;
        let mut counter = None;
        if self.eat(&TokenKind::LeftParen) {
            self.expect_keyword(Keyword::Let)?;
            let name = self.expect_identifier()?;
            if !self.check_keyword(Keyword::Where) {
                return Err(self.unexpected("`where` and the values to count through"));
            }
            let rules = self.parse_rules()?;
            self.expect(&TokenKind::RightParen)?;
            counter = Some(LoopCounter { name, rules });
        }
        let body = self.parse_block()?;
        Ok(Expr {
            kind: ExprKind::Loop(counter, Box::new(body)),
            span,
        })
    }
}
