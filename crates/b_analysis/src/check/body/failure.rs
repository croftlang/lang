//! Failing: `?` passes a failure up, and `or` fails with a chosen error.

use b_syntax::ast::Expr;
use b_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `value?`: the success value, or fail with the value's error.
    pub(super) fn try_value(&mut self, inner: &Expr) -> Type {
        match self.expr(inner, None) {
            Type::Result(value, error) => {
                self.check_error_passes(&error, inner.span);
                *value
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.error(format!("`?` needs a `Result`, found `{other}`"), inner.span);
                Type::Unknown
            }
        }
    }

    /// `value or Error`: the success value, or fail with `Error` instead.
    /// Works on a `Result`, an `Option`, and a `Bool` check.
    pub(super) fn or_else(&mut self, value: &Expr, error: &Expr) -> Type {
        let found = self.expr(value, None);
        self.check_failure(error);
        match found {
            Type::Result(value, _) | Type::Option(value) => *value,
            Type::Bool => Type::Unit,
            Type::Unknown => Type::Unknown,
            other => {
                self.error(
                    format!("`or` needs a `Result`, `Option` or `Bool`, found `{other}`"),
                    value.span,
                );
                Type::Unknown
            }
        }
    }

    /// The error after `or`, which the body must be able to fail with.
    pub(super) fn check_failure(&mut self, error: &Expr) {
        let expected = self.error.clone();
        let found = self.expr(error, expected.as_ref());
        let is_error =
            matches!(&found, Type::Named(n) if self.checker.is_error_type(n)) || found.is_unknown();
        if !is_error {
            self.error(
                format!("expected an error after `or`, found `{found}`"),
                error.span,
            );
            return;
        }
        self.check_error_passes(&found, error.span);
    }

    /// Whether the body's own error type can carry `error` up.
    pub(super) fn check_error_passes(&mut self, error: &Type, span: Span) {
        match self.error.clone() {
            None if !matches!(self.kind, BodyKind::Policy) => {
                self.error(
                    "this body cannot fail: give it a `Result` return type",
                    span,
                );
            }
            Some(own) if !self.checker.accepts(&own, error) => {
                let missing = match error {
                    Type::Errors(errors) => {
                        let missing = errors
                            .iter()
                            .filter(|e| !self.checker.accepts(&own, e))
                            .cloned();
                        Type::Errors(missing.collect())
                    }
                    other => other.clone(),
                };
                self.error(
                    format!("`{own}` does not include `{missing}`, so it cannot be passed up here"),
                    span,
                );
            }
            _ => {}
        }
    }
}
