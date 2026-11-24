//! `where` rules and the constraints they apply.

use super::{ParseResult, Parser};
use crate::ast::{ConstraintUse, Rule};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// `where Length(0, MAX_ENEMIES), each Alive`. A comma followed by
    /// `name:` starts the next parameter instead.
    pub(super) fn parse_rules(&mut self) -> ParseResult<Vec<Rule>> {
        let mut rules = Vec::new();
        if !self.eat_keyword(Keyword::Where) {
            return Ok(rules);
        }
        loop {
            let each = self.eat_keyword(Keyword::Each);
            rules.push(Rule {
                each,
                constraint: self.parse_constraint_use()?,
            });
            let next_is_rule = self.check(&TokenKind::Comma)
                && (self.peek_nth(1).kind == TokenKind::Keyword(Keyword::Each)
                    || self.peek_nth(2).kind != TokenKind::Colon);
            if !next_is_rule {
                return Ok(rules);
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
        Ok(ConstraintUse { name, arguments })
    }
}
