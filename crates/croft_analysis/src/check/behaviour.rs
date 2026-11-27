//! Functions, services, translations, commands, policies and constraints.

use croft_syntax::ast::{ConstraintDecl, FunctionDecl, FunctionKind, PolicyDecl};

use super::constraints::ConstraintPlace;
use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// Checks a function, service, translation, command or factory.
    pub(super) fn check_function(&mut self, scope: &Scope, function: &FunctionDecl) {
        let name = &function.name;
        let span = name.span;

        match &function.parameters {
            Some(parameters) => {
                self.check_parameters(scope, parameters, false, ConstraintPlace::Parameter)
            }
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
            (Some(_), FunctionKind::Factory) => {
                let message =
                    "a factory gives back the aggregate it makes, so it has no return type";
                self.error(message, span);
            }
            (Some(ty), _) => {
                self.check_type(scope, ty, false);
                let place = ConstraintPlace::Result;
                self.check_constraints(scope, &function.return_constraints, Some(ty), place);
            }
            (None, _) => {}
        }

        self.check_calls(scope, function);

        if function.body.is_none() {
            self.error(format!("`{}` needs a body after `=`", name.name), span);
        }
    }

    /// A policy's `when` names an event.
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

        self.check_parameters(
            scope,
            &constraint.parameters,
            true,
            ConstraintPlace::Declaration,
        );
    }
}
