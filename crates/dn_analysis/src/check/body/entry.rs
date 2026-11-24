//! Which bodies are checked, and what each starts with in scope.

use dn_syntax::ast::{FunctionDecl, FunctionKind, Parameter};

use super::{BodyChecker, BodyKind};
use crate::check::{Checker, Scope};
use crate::program::Context;
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'a> Checker<'a> {
    pub(in crate::check) fn check_bodies(&mut self, context: &'a Context) {
        let scope = Scope {
            context,
            aggregate: None,
        };
        for constant in &context.consts {
            let ty = self.lower_type(&scope, &constant.ty);
            BodyChecker::new(self, scope, BodyKind::Constant, ty.clone())
                .expect(&constant.value, &ty);
        }
        for constraint in &context.constraints {
            let mut body = BodyChecker::new(self, scope, BodyKind::Constraint, Type::Bool);
            body.bind_parameters(&constraint.parameters);
            body.expect(&constraint.body, &Type::Bool);
        }
        for function in &context.functions {
            let kind = match function.kind {
                FunctionKind::Service | FunctionKind::Domain => BodyKind::Service,
                _ => BodyKind::Function,
            };
            self.check_function_body(scope, kind, function);
        }
        for aggregate in &context.aggregates {
            let scope = Scope {
                context,
                aggregate: Some(&aggregate.name.name),
            };
            for command in &aggregate.commands {
                self.check_function_body(scope, BodyKind::Command(aggregate), command);
            }
        }
        for policy in &context.policies {
            let event = self
                .resolve(&scope, &policy.event)
                .map_or(Type::Unknown, Type::Named);
            let mut body = BodyChecker::new(self, scope, BodyKind::Policy, Type::Unknown);
            body.bind("event", event, false);
            body.expr(&policy.body, None);
        }
        for adapter in &context.adapters {
            self.check_adapter_bodies(scope, adapter);
        }
    }

    pub(super) fn check_function_body(
        &mut self,
        scope: Scope<'a>,
        kind: BodyKind<'a>,
        function: &FunctionDecl,
    ) {
        let result = match (kind, &function.return_type) {
            (BodyKind::Command(aggregate), _) => {
                let errors = format!("{}.Error", aggregate.name.name);
                let errors = self
                    .lookup(&scope.context.name.name, &errors)
                    .map_or(Type::Unknown, Type::Named);
                Type::result(Type::Unit, errors)
            }
            (_, Some(ty)) => self.lower_type(&scope, ty),
            // A domain's body can pass up any error; the program then stops with it.
            (_, None) if function.kind == FunctionKind::Domain => {
                Type::result(Type::Unit, Type::Errors(Vec::new()))
            }
            (_, None) => Type::Unit,
        };
        let mut body = BodyChecker::new(self, scope, kind, result.clone());
        body.bind_parameters(function.parameters.as_deref().unwrap_or(&[]));
        if matches!(function.kind, FunctionKind::Service | FunctionKind::Domain) {
            body.bind_uses(&function.uses);
        }
        for precondition in &function.preconditions {
            body.check_precondition(precondition);
        }
        if let Some(value) = &function.body {
            body.expect_body(value);
        }
    }

    pub(super) fn parameter_type(&self, scope: &Scope, parameter: &Parameter) -> Type {
        parameter
            .types
            .first()
            .map_or(Type::Unknown, |ty| self.lower_type(scope, ty))
    }
}

impl<'c, 'a> BodyChecker<'c, 'a> {}

/// The built-in port that gives a program its command-line arguments.
pub(super) fn arguments_port() -> Resolved {
    Resolved {
        context: String::new(),
        name: "Arguments".to_string(),
        kind: SymbolKind::Port,
    }
}
