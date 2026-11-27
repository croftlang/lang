//! What a body starts with: its parameters, the ports a service calls, its
//! contract lines, and the type its last line must have.

use croft_syntax::ast::{CalledPort, Expr, ExprKind, Parameter};

use super::BodyChecker;
use crate::naming::to_snake_case;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Brings each parameter into scope with its type.
    pub(super) fn bind_parameters(&mut self, parameters: &[Parameter]) {
        for parameter in parameters {
            let ty = self.checker.parameter_type(&self.scope, parameter);
            self.bind(&parameter.name.name, ty, false, Some(parameter.name.span));
        }
    }

    /// A service sees each port and repository it calls under the name its
    /// `calls` gives it: the declared name, or its alias.
    pub(super) fn bind_calls(&mut self, calls: &[CalledPort]) {
        for called in calls {
            let handle = match self.checker.find_called(&self.scope, called) {
                Some(found) if found.kind == SymbolKind::Repository => Type::Repository(found),
                Some(found) => Type::Port(found),
                None => Type::Unknown,
            };

            self.bind(&called.name.local_name(), handle, false, None);
        }
    }

    /// The name a port or repository in `calls` goes by, when the body wrote
    /// `name` for it instead: its declared name when an alias replaced it, or
    /// the snake_case name ports were once called by.
    pub(super) fn called_port_name(&self, name: &str) -> Option<String> {
        let mut names = self.locals.iter().flat_map(|scope| scope.iter());

        names.find_map(|(local, value)| match &value.ty {
            Type::Port(port) | Type::Repository(port) => {
                let declared = port.name.as_str();
                let means = name == declared || name == to_snake_case(declared);
                (means && local != name).then(|| local.clone())
            }
            _ => None,
        })
    }

    /// A line of a contract: a constraint such as `from != to`, or with `or`,
    /// `cond or Error { ... }`, failing with that error.
    pub(super) fn check_contract_line(&mut self, line: &Expr) {
        match &line.kind {
            ExprKind::OrElse(condition, error) => {
                self.expect(condition, &Type::Bool);
                self.check_failure(error);
            }
            _ => {
                self.expect(line, &Type::Bool);
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
