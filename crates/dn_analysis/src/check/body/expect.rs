//! Checking an expression against the type it must have.

use dn_syntax::ast::Expr;

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
    /// identity built on `Int`, so `const FIRST_ROOM: RoomId = 1` works.
    pub(super) fn accepts(&self, expected: &Type, found: &Type, value: &Expr) -> bool {
        let is_int_literal = matches!(value.kind, dn_syntax::ast::ExprKind::Int(_));
        if is_int_literal && found == &Type::Int && self.checker.is_int_identity(expected) {
            return true;
        }
        self.checker.accepts(expected, found)
    }
}
