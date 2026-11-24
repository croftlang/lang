//! Blocks, statements, guards and `match`.

use super::{ParseResult, Parser};
use crate::ast::{Expr, ExprKind, MatchArm, Pattern, Statement};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// `{ ... }`: one statement per line, the last line being the result.
    pub(super) fn parse_block(&mut self) -> ParseResult<Expr> {
        let span = self.span();
        self.expect(&TokenKind::LeftBrace)?;
        let statements = self.with_records(true, |parser| {
            let mut statements = Vec::new();
            while !parser.eat(&TokenKind::RightBrace) {
                if parser.check(&TokenKind::EndOfFile) {
                    return Err(parser.unexpected("`}`"));
                }
                statements.push(parser.parse_statement()?);
            }
            Ok(statements)
        })?;
        Ok(Expr {
            kind: ExprKind::Block(statements),
            span,
        })
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let keyword = self.peek().kind.clone();
        if keyword != TokenKind::Keyword(Keyword::Let)
            && keyword != TokenKind::Keyword(Keyword::Set)
        {
            return Ok(Statement::Expr(self.parse_expression()?));
        }
        self.advance();
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Equal)?;
        let value = self.parse_expression()?;
        if keyword == TokenKind::Keyword(Keyword::Let) {
            Ok(Statement::Let(name, value))
        } else {
            Ok(Statement::Set(name, value))
        }
    }

    /// `if condition { ... }`: a guard with no `else`.
    pub(super) fn parse_if(&mut self) -> ParseResult<Expr> {
        let span = self.span();
        self.expect_keyword(Keyword::If)?;
        let condition = self.with_records(false, Self::parse_expression)?;
        let block = self.parse_block()?;
        Ok(Expr {
            kind: ExprKind::If(Box::new(condition), Box::new(block)),
            span,
        })
    }

    /// `match kind { Coins(count) -> count * 10 }`
    pub(super) fn parse_match(&mut self) -> ParseResult<Expr> {
        let span = self.span();
        self.expect_keyword(Keyword::Match)?;
        let subject = self.with_records(false, Self::parse_expression)?;
        self.expect(&TokenKind::LeftBrace)?;
        let mut arms = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::Arrow)?;
            let value = self.with_records(true, Self::parse_expression)?;
            arms.push(MatchArm { pattern, value });
            self.eat(&TokenKind::Comma);
        }
        Ok(Expr {
            kind: ExprKind::Match(Box::new(subject), arms),
            span,
        })
    }

    /// `Coins(count)`, `Item.Gold(amount)` or `None`.
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let path = self.parse_path()?;
        let mut bindings = Vec::new();
        if self.eat(&TokenKind::LeftParen) {
            while !self.eat(&TokenKind::RightParen) {
                bindings.push(self.expect_identifier()?);
                self.eat(&TokenKind::Comma);
            }
        }
        Ok(Pattern { path, bindings })
    }
}
