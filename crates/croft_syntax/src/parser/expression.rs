//! Operators, from `or` down to calls and field access.
//!
//! A new line ends an expression, except before `.` and `?`, so a chain of
//! calls can be split one per line.

use super::{ParseResult, Parser};
use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// A whole expression, `or` included.
    pub(super) fn parse_expression(&mut self) -> ParseResult<Expr> {
        let mut value = self.parse_or_operand()?;

        while self.check_keyword(Keyword::Or) && !self.peek().newline_before {
            self.advance();

            let fallback = self.parse_or_operand()?;
            let span = value.span;
            value = self.make_expr(ExprKind::OrElse(Box::new(value), Box::new(fallback)), span);
        }

        Ok(value)
    }

    /// An expression without `or`, such as the error after an invariant.
    pub(super) fn parse_or_operand(&mut self) -> ParseResult<Expr> {
        self.parse_binary(1)
    }

    /// Binary operators that bind at least as tightly as `min_precedence`.
    fn parse_binary(&mut self, min_precedence: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;

        while let Some((op, precedence)) = self.binary_operator() {
            if precedence < min_precedence {
                break;
            }

            self.advance();

            let right = self.parse_binary(precedence + 1)?;
            let span = left.span;
            left = self.make_expr(ExprKind::Binary(op, Box::new(left), Box::new(right)), span);
        }

        Ok(left)
    }

    /// The binary operator next, with its precedence. None when a new line
    /// comes first, since a new line ends the expression.
    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        if self.peek().newline_before {
            return None;
        }

        let op = match self.peek().kind {
            TokenKind::Or => BinaryOp::Or,
            TokenKind::And => BinaryOp::And,
            TokenKind::EqualEqual => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            TokenKind::Pipe => BinaryOp::BitOr,
            TokenKind::Caret => BinaryOp::BitXor,
            TokenKind::Ampersand => BinaryOp::BitAnd,
            TokenKind::ShiftLeft => BinaryOp::ShiftLeft,
            TokenKind::ShiftRight => BinaryOp::ShiftRight,
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            TokenKind::Percent => BinaryOp::Remainder,
            _ => return None,
        };

        Some((op, op.precedence()))
    }

    /// `!`, `-` or `~` before an operand, or a postfix expression.
    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let span = self.span();

        let op = match self.peek().kind {
            TokenKind::Not => UnaryOp::Not,
            TokenKind::Minus => UnaryOp::Negate,
            TokenKind::Tilde => UnaryOp::BitNot,
            _ => return self.parse_postfix(true),
        };

        self.advance();

        let operand = self.parse_unary()?;

        Ok(self.make_expr(ExprKind::Unary(op, Box::new(operand)), span))
    }
}
