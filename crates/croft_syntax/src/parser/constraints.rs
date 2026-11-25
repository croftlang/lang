//! `where` clauses and the constraints they apply.

use super::{ParseResult, Parser};
use crate::ast::ConstraintUse;
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// `where Length(0, MAX_ENEMIES), each Alive`. A comma followed by
    /// `name:` starts the next parameter instead.
    pub(super) fn parse_constraints(&mut self) -> ParseResult<Vec<ConstraintUse>> {
        let mut constraints = Vec::new();

        if !self.eat_keyword(Keyword::Where) {
            return Ok(constraints);
        }

        loop {
            let each = self.eat_keyword(Keyword::Each);
            let mut constraint = self.parse_constraint_use()?;
            constraint.each = each;
            constraints.push(constraint);

            let next_is_constraint = self.check(&TokenKind::Comma)
                && (self.peek_nth(1).kind == TokenKind::Keyword(Keyword::Each)
                    || self.peek_nth(2).kind != TokenKind::Colon);

            if !next_is_constraint {
                return Ok(constraints);
            }

            self.advance();
        }
    }

    /// `Range(1, 20)` or just `Alive`.
    pub(super) fn parse_constraint_use(&mut self) -> ParseResult<ConstraintUse> {
        let name = self.parse_path()?;
        let mut arguments = Vec::new();

        if self.check(&TokenKind::LeftParen) && !self.peek().newline_before {
            arguments = self
                .parse_arguments()?
                .into_iter()
                .map(|a| a.value)
                .collect();
        }

        Ok(ConstraintUse {
            each: false,
            name,
            arguments,
        })
    }
}
