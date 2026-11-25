//! Call arguments and the field values of records and `with`.

use super::{ParseResult, Parser};
use crate::ast::{Argument, FieldValue};
use crate::token::TokenKind;

impl Parser {
    /// `(start, amount: 100)`: each argument is positional or named.
    pub(super) fn parse_arguments(&mut self) -> ParseResult<Vec<Argument>> {
        self.expect(&TokenKind::LeftParen)?;
        let mut arguments = Vec::new();
        while !self.eat(&TokenKind::RightParen) {
            let named = self.is_identifier() && self.peek_nth(1).kind == TokenKind::Colon;
            let name = if named {
                Some(self.expect_identifier()?)
            } else {
                None
            };
            if named {
                self.advance();
            }
            let value = self.with_records(true, Self::parse_expression)?;
            arguments.push(Argument { name, value });
            if !self.eat(&TokenKind::Comma) {
                self.expect(&TokenKind::RightParen)?;
                break;
            }
        }
        Ok(arguments)
    }

    /// `{ x: 10, y: 20 }`, or `{ enemy_id }` as shorthand for `{ enemy_id: enemy_id }`.
    pub(super) fn parse_field_values(&mut self) -> ParseResult<Vec<FieldValue>> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut values = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            let name = self.expect_field_name()?;
            let value = if self.eat(&TokenKind::Colon) {
                Some(self.with_records(true, Self::parse_expression)?)
            } else {
                None
            };
            values.push(FieldValue { name, value });
            self.eat(&TokenKind::Comma);
        }
        Ok(values)
    }

    /// Runs `parse` with record literals allowed or not, then restores the setting.
    pub(super) fn with_records<T>(
        &mut self,
        allow: bool,
        parse: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let previous = std::mem::replace(&mut self.allow_records, allow);
        let result = parse(self);
        self.allow_records = previous;
        result
    }

    /// `[a, b, c]`: items separated by commas, across lines if need be.
    pub(super) fn parse_list(&mut self) -> ParseResult<Vec<crate::ast::Expr>> {
        self.expect(&TokenKind::LeftBracket)?;
        let mut items = Vec::new();
        while !self.eat(&TokenKind::RightBracket) {
            items.push(self.with_records(true, Self::parse_expression)?);
            if !self.eat(&TokenKind::Comma) {
                self.expect(&TokenKind::RightBracket)?;
                break;
            }
        }
        Ok(items)
    }
}
