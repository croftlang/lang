//! What a body starts with: its parameters, the ports a service calls, its
//! preconditions, and the type its last line must have.

use b_syntax::ast::{Expr, ExprKind, Parameter};

use super::entry::builtin_port;
use super::BodyChecker;
use crate::naming::to_snake_case;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn bind_parameters(&mut self, parameters: &[Parameter]) {
        for parameter in parameters {
            let ty = self.checker.parameter_type(&self.scope, parameter);
            self.bind(&parameter.name.name, ty, false, Some(parameter.name.span));
        }
    }

    /// A service sees each port and repository it calls under its snake_case name.
    pub(super) fn bind_calls(&mut self, calls: &[b_syntax::ast::Path]) {
        for path in calls {
            let handle = match self.checker.resolve(&self.scope, path) {
                Some(found) if found.kind == SymbolKind::Repository => Type::Repository(found),
                Some(found) => Type::Port(found),
                None => Type::Port(builtin_port(&path.last().name)),
            };
            self.bind(&to_snake_case(&path.last().name), handle, false, None);
        }
    }

    /// `cond`, or `cond or Error { ... }` to fail with that error.
    pub(super) fn check_precondition(&mut self, precondition: &Expr) {
        match &precondition.kind {
            ExprKind::OrElse(condition, error) => {
                self.expect(condition, &Type::Bool);
                self.check_failure(error);
            }
            _ => {
                self.expect(precondition, &Type::Bool);
            }
        }
    }

    /// A body must give back its declared type; a plain value counts as `Ok`.
    pub(super) fn expect_body(&mut self, value: &Expr) {
        let result = self.result.clone();
        if result.is_unknown() {
            self.expr(value, None);
        } else if let Type::Result(inner, _) = &result {
            let inner = (**inner).clone();
            let found = self.expr(value, Some(&result));
            if !self.accepts(&result, &found, value) && !self.accepts(&inner, &found, value) {
                self.error(format!("expected `{result}`, found `{found}`"), value.span);
            }
        } else {
            self.expect(value, &result);
        }
    }
}
