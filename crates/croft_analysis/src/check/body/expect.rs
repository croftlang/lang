//! Checking an expression against the type it must have.

use croft_syntax::ast::{Expr, ExprKind};

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks an expression against the type it must have, and reports a mismatch.
    pub(super) fn expect(&mut self, value: &Expr, expected: &Type) -> Type {
        let found = self.expr(value, Some(expected));
        if !self.accepts(expected, &found, value) {
            self.error(
                format!("expected `{expected}`, found `{found}`"),
                value.span,
            );
        }
        found
    }

    /// Like `Checker::accepts`, and a whole-number literal also counts as an
    /// identity built on `Int`, so `const FIRST_ROOM: RoomId = 1` works, and
    /// any literal as a single-value type around its type.
    pub(super) fn accepts(&mut self, expected: &Type, found: &Type, value: &Expr) -> bool {
        let is_int_literal = matches!(value.kind, croft_syntax::ast::ExprKind::Int(_));
        if is_int_literal && found == &Type::Int && self.checker.is_int_identity(expected) {
            return true;
        }
        // A literal is also a single-value type around its type: `let best: Score = 100`.
        let is_literal = matches!(
            value.kind,
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Bool(_)
        );
        if let Some(inner) = self.checker.single_value_inner(expected) {
            if is_literal && self.checker.accepts(&inner, found) {
                self.checker.facts.wanted.insert(value.id, expected.clone());
                return true;
            }
        }
        self.checker.accepts(expected, found)
    }
}
