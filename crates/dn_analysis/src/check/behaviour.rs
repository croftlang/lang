//! Functions, services, translations, commands, policies and constraints.

use dn_syntax::ast::{ConstraintDecl, FunctionDecl, FunctionKind, PolicyDecl};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// Checks a function, service, translation or command.
    pub(super) fn check_function(&mut self, scope: &Scope, function: &FunctionDecl) {
        let name = &function.name;
        let span = name.span;
        match &function.parameters {
            Some(parameters) => self.check_parameters(scope, parameters, false),
            None if function.kind != FunctionKind::Command => {
                self.error(
                    format!("`{}` needs brackets, even with no parameters", name.name),
                    span,
                );
            }
            None => {}
        }
        match (&function.return_type, function.kind) {
            (Some(_), FunctionKind::Command) => {
                let message = "a command gives back its aggregate, so it has no return type";
                self.error(message, span);
            }
            (None, FunctionKind::Function | FunctionKind::Translation) => {
                self.error(format!("`{}` needs a return type", name.name), span);
            }
            (Some(ty), _) => {
                self.check_type(scope, ty, false);
                self.check_rules(scope, &function.return_rules, Some(ty));
            }
            (None, _) => {}
        }
        if matches!(function.kind, FunctionKind::Service | FunctionKind::Domain) {
            let kinds = [SymbolKind::Port, SymbolKind::Repository];
            for port in &function.uses {
                let is_builtin = port.text() == "Arguments" && self.resolve(scope, port).is_none();
                if !is_builtin {
                    self.expect_kind(scope, port, &kinds, "a port or repository");
                }
            }
        } else if let Some(port) = function.uses.first() {
            self.error("only a service can use ports", port.span());
        }
        if function.body.is_none() {
            self.error(format!("`{}` needs a body after `=`", name.name), span);
        }
    }

    pub(super) fn check_policy(&mut self, scope: &Scope, policy: &PolicyDecl) {
        self.expect_kind(scope, &policy.event, &[SymbolKind::Event], "an event");
    }

    /// The first parameter of a constraint is the value it checks.
    pub(super) fn check_constraint(&mut self, scope: &Scope, constraint: &ConstraintDecl) {
        if constraint.parameters.is_empty() {
            let message = format!(
                "constraint `{}` needs the value it checks as its first parameter",
                constraint.name.name
            );
            self.error(message, constraint.name.span);
        }
        self.check_parameters(scope, &constraint.parameters, true);
    }
}
