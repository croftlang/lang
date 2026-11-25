//! Operators, from `or` down to calls and field access.
//!
//! A new line ends an expression, except before `.` and `?`, so a chain of
//! calls can be split one per line.

use super::{ParseResult, Parser};
use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
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

    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        if self.peek().newline_before {
            return None;
        }
        // Bit operators bind tighter than comparisons, so `bits & FIRE != 0`
        // means `(bits & FIRE) != 0`, unlike in C.
        let operator = match self.peek().kind {
            TokenKind::Or => (BinaryOp::Or, 1),
            TokenKind::And => (BinaryOp::And, 2),
            TokenKind::EqualEqual => (BinaryOp::Equal, 3),
            TokenKind::NotEqual => (BinaryOp::NotEqual, 3),
            TokenKind::Less => (BinaryOp::Less, 4),
            TokenKind::LessEqual => (BinaryOp::LessEqual, 4),
            TokenKind::Greater => (BinaryOp::Greater, 4),
            TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, 4),
            TokenKind::Pipe => (BinaryOp::BitOr, 5),
            TokenKind::Caret => (BinaryOp::BitXor, 6),
            TokenKind::Ampersand => (BinaryOp::BitAnd, 7),
            TokenKind::ShiftLeft => (BinaryOp::ShiftLeft, 8),
            TokenKind::ShiftRight => (BinaryOp::ShiftRight, 8),
            TokenKind::Plus => (BinaryOp::Add, 9),
            TokenKind::Minus => (BinaryOp::Subtract, 9),
            TokenKind::Star => (BinaryOp::Multiply, 10),
            TokenKind::Slash => (BinaryOp::Divide, 10),
            TokenKind::Percent => (BinaryOp::Remainder, 10),
            _ => return None,
        };
        Some(operator)
    }

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
