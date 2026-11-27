//! Blocks and their statements: `let`, `set`, guards, loops and `break`.

use croft_syntax::ast::{ConstraintUse, Expr, ExprKind, Identifier, Statement, TypeExpr};

use super::let_constraints::only_adds_to;
use super::{BodyChecker, BodyKind};
use crate::check::known::Known;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A block's type is its last line's, or `Unit` when that line is a step.
    pub(super) fn block(&mut self, statements: &[Statement], expected: Option<&Type>) -> Type {
        super::known_constraints::changed_names(statements, &mut self.changed);

        self.nested(|body| {
            let mut result = Type::Unit;

            for (index, statement) in statements.iter().enumerate() {
                let is_last = index + 1 == statements.len();

                result = match statement {
                    Statement::Let {
                        name,
                        ty,
                        constraints,
                        value,
                    } => {
                        let rest = &statements[index + 1..];
                        body.check_let(name, ty.as_ref(), constraints, value, rest)
                    }
                    Statement::Set(name, value) => body.check_set(name, value),
                    Statement::Expr(value) => body.check_line(value, is_last, expected),
                };
            }

            result
        })
    }

    /// `let`: names a value, with the type written or the value's own, and
    /// the constraints it must keep. Only a service's `let` can be changed
    /// with `set`; `rest` is the block after it, where those `set`s are.
    pub(super) fn check_let(
        &mut self,
        name: &Identifier,
        written: Option<&TypeExpr>,
        constraints: &[ConstraintUse],
        value: &Expr,
        rest: &[Statement],
    ) -> Type {
        if self.local(&name.name).is_some() {
            self.error(
                format!("`{}` is already named in this body", name.name),
                name.span,
            );
        }

        let ty = match written {
            Some(written) => {
                let wanted = self.lower_written(written);
                let found = self.expect(value, &wanted);

                if found != wanted {
                    self.checker.facts.wanted.insert(value.id, wanted.clone());
                }

                wanted
            }
            None => self.expr(value, None),
        };

        let settable = matches!(self.kind, BodyKind::Service);
        let changed = settable && self.changed.contains(&name.name);
        let known = self.known_local_value(value);

        self.check_let_constraints(name, &ty, constraints, value, known.as_ref());

        if changed {
            self.require_length(name, written, &ty, constraints);
        }

        // A set doesn't grow when it is given an item it already holds.
        let grows_from = (changed && matches!(ty, Type::List(_)) && only_adds_to(rest, &name.name))
            .then(|| known.as_ref().and_then(Known::length))
            .flatten()
            .and_then(|length| match length {
                Known::Number { units, .. } => Some(units),
                _ => None,
            });

        self.bind(&name.name, ty, settable, Some(name.span));

        let Some(local) = self
            .locals
            .last_mut()
            .and_then(|scope| scope.get_mut(&name.name))
        else {
            return Type::Unit;
        };

        local.constraints = constraints.to_vec();
        local.grows_from = grows_from;

        // A service's `let` can be changed with `set`, so its value is only
        // known when nothing in the body changes it.
        if !changed {
            local.known = known;
        }

        Type::Unit
    }

    /// `set` changes a command's aggregate field, an adapter's field, or a
    /// service's own `let` name, and nothing else.
    pub(super) fn check_set(&mut self, name: &Identifier, value: &Expr) -> Type {
        let target = match self.kind {
            BodyKind::Command(_) | BodyKind::Adapter(_) => {
                let found = self.own_field(&name.name, name.span, true);

                if let Some(ty) = &found {
                    self.note_own_field(&name.name, name.span, ty);
                }

                found
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
                self.check_set_constraints(name, value);
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

    /// A type written in a body, such as on a `let`, reported if it is wrong.
    fn lower_written(&mut self, written: &TypeExpr) -> Type {
        self.checker.check_type(&self.scope, written, false);
        self.checker.lower_type(&self.scope, written)
    }
}
