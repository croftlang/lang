//! Which bodies are checked, and what each starts with in scope.

use croft_syntax::ast::{FunctionDecl, FunctionKind, Parameter};

use super::{BodyChecker, BodyKind};
use crate::check::{Checker, Scope};
use crate::program::Context;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// Checks every body in a context: constants, constraints, functions,
    /// services, commands, factories, policies, adapters and the contracts
    /// of ports.
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

        // A generic template's starting values are checked in each instance.
        for value in context
            .values
            .iter()
            .filter(|v| v.type_parameters.is_empty())
        {
            self.check_starting_values(scope, &value.name.name, &value.fields);
        }

        for error in &context.errors {
            self.check_starting_values(scope, &error.name.name, &error.fields);
        }

        for constraint in &context.constraints {
            let mut body = BodyChecker::new(self, scope, BodyKind::Constraint, Type::Bool);
            body.bind_parameters(&constraint.parameters);
            body.expect(&constraint.body, &Type::Bool);
        }

        // A generic template is checked as each of its instances.
        for function in context
            .functions
            .iter()
            .filter(|f| f.type_parameters.is_empty())
        {
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

            let name = &aggregate.name.name;
            self.check_starting_values(scope, name, &aggregate.fields);

            for entity in &aggregate.entities {
                let owner = format!("{name}.{}", entity.name.name);
                self.check_starting_values(scope, &owner, &entity.fields);
            }

            for record in aggregate.errors.iter().chain(&aggregate.events) {
                let owner = format!("{name}.{}", record.name.name);
                self.check_starting_values(scope, &owner, &record.fields);
            }

            for command in &aggregate.commands {
                self.check_function_body(scope, BodyKind::Command(aggregate), command);
            }

            for factory in &aggregate.factories {
                self.check_function_body(scope, BodyKind::Factory(aggregate), factory);
            }
        }

        for policy in &context.policies {
            let event = self
                .resolve(&scope, &policy.event)
                .map_or(Type::Unknown, Type::Named);

            let mut body = BodyChecker::new(self, scope, BodyKind::Policy, Type::Unknown);
            body.bind("event", event, false, None);
            body.expr(&policy.body, None);
        }

        for adapter in &context.adapters {
            self.check_adapter_bodies(scope, adapter);
        }

        // A port operation's contract reads only its parameters.
        for operation in context.ports.iter().flat_map(|port| &port.operations) {
            let parameters = operation.parameters.as_deref().unwrap_or(&[]);
            let mut body = BodyChecker::new(self, scope, BodyKind::Function, Type::Bool);
            body.bind_parameters(parameters);
            body.check_constraint_arguments(parameters);

            for line in &operation.preconditions {
                body.expect(line, &Type::Bool);
            }
        }
    }

    /// Checks a function, service, command or factory body against the type it
    /// gives back and the errors it can fail with.
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
            (BodyKind::Factory(aggregate), _) => {
                let made = self
                    .lookup(&scope.context.name.name, &aggregate.name.name)
                    .map_or(Type::Unknown, Type::Named);

                let errors = format!("{}.Error", aggregate.name.name);

                let errors = self
                    .lookup(&scope.context.name.name, &errors)
                    .map_or(Type::Unknown, Type::Named);

                Type::result(made, errors)
            }
            (_, Some(ty)) => self.lower_type(&scope, ty),
            // A domain's body can pass up any error; the program then stops with it.
            (_, None) if function.kind == FunctionKind::Domain => {
                Type::result(Type::Unit, Type::Errors(Vec::new()))
            }
            (_, None) => Type::Unit,
        };

        let parameters = function.parameters.as_deref().unwrap_or(&[]);

        // A result's constraints read no parameters: they are checked where
        // the result is given back, once the parameters are gone.
        let mut results = BodyChecker::new(self, scope, BodyKind::Constant, result.clone());

        for constraint in &function.return_constraints {
            results.check_result_constraint_arguments(constraint, &result, parameters);
        }

        let mut body = BodyChecker::new(self, scope, kind, result.clone());
        body.bind_parameters(parameters);
        body.check_constraint_arguments(parameters);

        if matches!(function.kind, FunctionKind::Service | FunctionKind::Domain) {
            body.bind_calls(&function.calls);
        }

        for precondition in &function.preconditions {
            body.check_contract_line(precondition);
        }

        // A parameter's constraint with `or` fails the call with its error.
        for parameter in function.parameters.iter().flatten() {
            for error in parameter
                .constraints
                .iter()
                .filter_map(|c| c.error.as_ref())
            {
                body.check_failure(error);
            }
        }

        match (&function.body, kind, &result) {
            // A factory's body is the aggregate itself; `?` and `or` fail the factory.
            (Some(value), BodyKind::Factory(_), Type::Result(made, _)) => {
                body.expect(value, made);
            }
            (Some(value), _, _) => body.expect_body(value),
            (None, _, _) => {}
        }
    }

    /// The type of a parameter, from its first written type.
    pub(crate) fn parameter_type(&self, scope: &Scope, parameter: &Parameter) -> Type {
        parameter
            .types
            .first()
            .map_or(Type::Unknown, |ty| self.lower_type(scope, ty))
    }
}

impl<'c, 'a> BodyChecker<'c, 'a> {}
