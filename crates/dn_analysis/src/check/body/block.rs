//! Blocks and their statements: `let`, `set`, guards, loops and `break`.

use dn_syntax::ast::{Expr, ExprKind, Identifier, Statement};

use super::{BodyChecker, BodyKind};
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A block's type is its last line's, or `Unit` when that line is a step.
    pub(super) fn block(&mut self, statements: &[Statement], expected: Option<&Type>) -> Type {
        self.nested(|body| {
            let mut result = Type::Unit;
            for (index, statement) in statements.iter().enumerate() {
                let is_last = index + 1 == statements.len();
                result = match statement {
                    Statement::Let(name, value) => body.check_let(name, value),
                    Statement::Set(name, value) => body.check_set(name, value),
                    Statement::Expr(value) => body.check_line(value, is_last, expected),
                };
            }
            result
        })
    }

    pub(super) fn check_let(&mut self, name: &Identifier, value: &Expr) -> Type {
        if self.local(&name.name).is_some() {
            self.error(
                format!("`{}` is already named in this body", name.name),
                name.span,
            );
        }
        let ty = self.expr(value, None);
        let settable = matches!(self.kind, BodyKind::Service);
        self.bind(&name.name, ty, settable);
        Type::Unit
    }

    /// `set` changes a command's aggregate field, an adapter's field, or a
    /// service's own `let` name, and nothing else.
    pub(super) fn check_set(&mut self, name: &Identifier, value: &Expr) -> Type {
        let target = match self.kind {
            BodyKind::Command(_) | BodyKind::Adapter(_) => {
                self.own_field(&name.name, name.span, true)
            }
            BodyKind::Service => match self.local(&name.name) {
                Some(local) if local.settable => Some(local.ty.clone()),
                _ => None,
            },
            _ => {
                let message = "`set` only works in commands, adapters and services";
                self.error(message, name.span);
                return Type::Unit;
            }
        };
        match target {
            Some(ty) => {
                self.expect(value, &ty);
            }
            None => {
                self.error(
                    format!("`set` cannot change `{}` here", name.name),
                    name.span,
                );
                self.expr(value, None);
            }
        }
        Type::Unit
    }

    /// One line of a block. A `Result` must be handled unless it is the result.
    pub(super) fn check_line(
        &mut self,
        value: &Expr,
        is_last: bool,
        expected: Option<&Type>,
    ) -> Type {
        if let ExprKind::If(condition, block) = &value.kind {
            self.check_guard(condition, block);
            return Type::Unit;
        }
        let ty = self.expr(value, if is_last { expected } else { None });
        let is_policy = matches!(self.kind, BodyKind::Policy);
        if matches!(ty, Type::Result(..)) && !is_last && !is_policy {
            let message = "this `Result` is not handled: add `?`, or `or` and an error";
            self.error(message, value.span);
        }
        ty
    }

    /// A guard whose block gives back a value ends the body with it. One that
    /// only takes steps runs them and the body carries on.
    pub(super) fn check_guard(&mut self, condition: &Expr, block: &Expr) {
        self.expect(condition, &Type::Bool);
        let ExprKind::Block(statements) = &block.kind else {
            return;
        };
        let result = self.result.clone();
        let expected = (!result.is_unknown()).then_some(&result);
        let found = self.block(statements, expected);
        if found == Type::Unit || result.is_unknown() {
            return;
        }
        let inner = match &result {
            Type::Result(inner, _) => (**inner).clone(),
            other => other.clone(),
        };
        if !self.checker.accepts(&result, &found) && !self.checker.accepts(&inner, &found) {
            let message =
                format!("this guard gives back `{found}`, but the body gives back `{result}`");
            self.error(message, block.span);
        }
    }
}
