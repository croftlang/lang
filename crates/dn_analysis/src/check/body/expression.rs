//! The type of each kind of expression, and operators.

use dn_syntax::ast::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of an expression. `expected`, when known, lets bare enum
    /// variants such as `North` be found.
    pub(super) fn expr(&mut self, value: &Expr, expected: Option<&Type>) -> Type {
        let ty = self.expr_kind(value, expected);
        self.checker.facts.types.insert(value.id, ty.clone());
        ty
    }

    fn expr_kind(&mut self, value: &Expr, expected: Option<&Type>) -> Type {
        let span = value.span;
        match &value.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::Float(_) => Type::Float,
            ExprKind::String(_) => Type::String,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Name(name) => self.name(Some(value.id), name, span, expected),
            ExprKind::Field(inner, field) => self.field(value, inner, field, expected),
            ExprKind::Call(callee, arguments) => {
                self.call(value.id, callee, arguments, span, expected)
            }
            ExprKind::Record(path, fields) => self.record(path, fields),
            ExprKind::With(base, fields) => self.with(base, fields),
            ExprKind::Unary(op, operand) => self.unary(*op, operand),
            ExprKind::Binary(op, left, right) => self.binary(*op, left, right),
            ExprKind::OrElse(value, error) => self.or_else(value, error),
            ExprKind::Try(inner) => self.try_value(inner),
            ExprKind::Lambda(..) => {
                self.error(
                    "a lambda can only be passed to a list or option method",
                    span,
                );
                Type::Unknown
            }
            ExprKind::If(condition, block) => {
                self.expect(condition, &Type::Bool);
                self.expr(block, None);
                Type::Unit
            }
            ExprKind::Match(subject, arms) => self.check_match(subject, arms, expected),
            ExprKind::Block(statements) => self.block(statements, expected),
            ExprKind::Issue(issue) => self.issue(value.id, issue, span),
            ExprKind::Emit(event, fields) => self.emit(event, fields, span),
            ExprKind::Native(function, arguments) => {
                self.native(function, arguments, span, expected)
            }
            ExprKind::Loop(counter, block) => self.check_loop(counter.as_ref(), block, span),
            ExprKind::Break => self.check_break(span),
        }
    }

    pub(super) fn unary(&mut self, op: UnaryOp, operand: &Expr) -> Type {
        match op {
            UnaryOp::Not => {
                self.expect(operand, &Type::Bool);
                Type::Bool
            }
            UnaryOp::Negate => {
                let ty = self.expr(operand, None);
                if !ty.is_number() {
                    self.error(format!("cannot negate `{ty}`"), operand.span);
                }
                ty
            }
        }
    }

    pub(super) fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Type {
        use BinaryOp::*;
        match op {
            And | Or => {
                self.expect(left, &Type::Bool);
                self.expect(right, &Type::Bool);
                Type::Bool
            }
            Equal | NotEqual => {
                let ty = self.expr(left, None);
                self.expect(right, &ty);
                Type::Bool
            }
            Less | LessEqual | Greater | GreaterEqual => {
                let ty = self.expr(left, None);
                if !ty.is_number() && !self.checker.is_int_identity(&ty) {
                    self.error(format!("cannot compare `{ty}` by size"), left.span);
                }
                self.expect(right, &ty);
                Type::Bool
            }
            Add | Subtract | Multiply | Divide | Remainder => {
                let ty = self.expr(left, None);
                let joins_strings = op == Add && ty == Type::String;
                if !ty.is_number() && !joins_strings {
                    self.error(format!("cannot do arithmetic on `{ty}`"), left.span);
                    return Type::Unknown;
                }
                self.expect(right, &ty);
                ty
            }
        }
    }
}
