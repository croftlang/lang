//! Calls: functions, translations, services, constraints, factories, enum
//! variants and built-in functions. Method calls are in `method`.

use croft_syntax::ast::{Argument, Expr, ExprId, ExprKind};
use croft_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::check::data::expr_path;
use crate::facts::{BuiltinName, Meaning};
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a call: a method, a declaration, a variant or a built-in function.
    pub(super) fn call(
        &mut self,
        id: ExprId,
        callee: &Expr,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        if let ExprKind::Field(receiver, method) = &callee.kind {
            if let ExprKind::Name(name) = &receiver.kind {
                if self.local(name).is_none() {
                    if let Some(called) = self.called_port_name(name) {
                        let message =
                            format!("cannot find `{name}` here: `calls` names it `{called}`");
                        self.error(message, receiver.span);

                        for argument in arguments {
                            self.expr(&argument.value, None);
                        }

                        return Type::Unknown;
                    }
                }
            }

            if self.as_type_path(callee).is_none() {
                let receiver = self.expr(receiver, None);
                self.note(Some(id), Meaning::Method);
                return self.method(&receiver, method, arguments, expected);
            }
        }

        let Some(path) = expr_path(callee) else {
            self.error("only a named function can be called", callee.span);
            return Type::Unknown;
        };

        let name = path.text();

        let Some(found) = self.checker.resolve(&self.scope, &path) else {
            if let Some(ty) = self.builtin_call(&name, arguments, span, expected) {
                self.note(Some(id), Meaning::Builtin(name.clone()));

                self.checker.facts.builtins.push(BuiltinName {
                    span: path.last().span,
                    name: name.clone(),
                });

                return ty;
            }

            if let Some((ty, variant)) = self.find_variant(&path, expected) {
                let parameters: Vec<(String, Type)> = (variant.fields.iter())
                    .map(|field| {
                        (
                            field.name.name.clone(),
                            self.checker.lower_type(&variant.scope, &field.ty),
                        )
                    })
                    .collect();

                self.check_arguments(arguments, &parameters, &name, span, false);

                let constraints = Self::field_constraints(variant.fields);
                let variant_name = path.last().name.clone();
                self.check_known_arguments(arguments, &constraints, &variant.scope, &variant_name);

                if let Type::Named(owner) = &ty {
                    self.note(
                        Some(id),
                        Meaning::Variant(
                            owner.clone(),
                            variant.name.clone(),
                            variant.error.clone(),
                        ),
                    );
                }

                return ty;
            }

            self.error(format!("cannot find `{name}` here"), path.span());
            return Type::Unknown;
        };

        if self.checker.generic_function(&found).is_some() {
            return self.call_generic(id, &found, arguments, span, expected);
        }

        self.note(Some(id), Meaning::Declared(found.clone()));

        match found.kind {
            SymbolKind::Function | SymbolKind::Translation | SymbolKind::Constraint => {
                self.call_declared(&found, arguments, span)
            }
            SymbolKind::Service if matches!(self.kind, BodyKind::Service) => {
                self.call_declared(&found, arguments, span)
            }
            SymbolKind::Service => {
                self.error("only a service can run another service", span);
                Type::Unknown
            }
            SymbolKind::Factory => self.call_factory(&found, arguments, span),
            SymbolKind::Value => match self.checker.single_value_inner(&Type::Named(found.clone()))
            {
                Some(inner) => {
                    let parameters = [("value".to_string(), inner)];
                    self.check_arguments(arguments, &parameters, &name, span, false);

                    let owner = self.checker.scope_of(&found);
                    let fields = (owner.context.values.iter())
                        .find(|value| value.name.name == found.name)
                        .map_or(&[][..], |value| value.fields.as_slice());

                    let constraints = Self::field_constraints(fields);
                    self.check_known_arguments(arguments, &constraints, &owner, &found.name);

                    Type::Named(found)
                }
                None => {
                    let message = format!("`{name}` is made with its fields: `{name} {{ ... }}`");
                    self.error(message, span);
                    Type::Unknown
                }
            },
            SymbolKind::Aggregate => {
                let message = format!(
                    "an aggregate is loaded by issuing a command: `issue Command on {name}(id)`"
                );

                self.error(message, span);

                Type::Unknown
            }
            kind => {
                self.error(
                    format!("`{name}` is {}, so it cannot be called", kind.describe()),
                    span,
                );

                Type::Unknown
            }
        }
    }

    /// `Cargo.Book(origin: ...)`: makes a new aggregate, or fails with one of
    /// its errors. Only its own context makes its aggregates.
    fn call_factory(&mut self, found: &Resolved, arguments: &[Argument], span: Span) -> Type {
        if !matches!(
            self.kind,
            BodyKind::Service | BodyKind::Policy | BodyKind::Function
        ) {
            self.error(
                format!(
                    "only services, policies and functions can make an aggregate with `{}`",
                    found.name
                ),
                span,
            );
        }

        if found.context != self.scope.context.name.name {
            let message = format!(
                "`{}` makes an aggregate of `{}`, so only `{}` can call it",
                found.name, found.context, found.context
            );

            self.error(message, span);
        }

        let Some((aggregate, factory)) = found.name.split_once('.') else {
            return Type::Unknown;
        };

        let scope = self.checker.scope_of(found);

        let declared = (scope.context.aggregates.iter())
            .find(|a| a.name.name == aggregate)
            .and_then(|a| a.factories.iter().find(|f| f.name.name == factory));

        let parameters: Vec<(String, Type)> = (declared.iter())
            .flat_map(|f| f.parameters.iter().flatten())
            .map(|p| (p.name.name.clone(), self.checker.parameter_type(&scope, p)))
            .collect();

        self.check_arguments(arguments, &parameters, factory, span, true);

        if let Some(declared) = declared {
            self.check_known_contract(declared, &scope, arguments, factory, span);
        }

        let declared = declared
            .and_then(|f| f.parameters.as_deref())
            .unwrap_or(&[]);
        let constraints = Self::parameter_constraints(declared);
        self.check_known_arguments(arguments, &constraints, &scope, factory);

        let made = self
            .checker
            .lookup(&found.context, aggregate)
            .map_or(Type::Unknown, Type::Named);

        let errors = self
            .checker
            .lookup(&found.context, &format!("{aggregate}.Error"))
            .map_or(Type::Unknown, Type::Named);

        Type::result(made, errors)
    }

    /// A call to a function, translation, service or constraint declared in Croft.
    pub(super) fn call_declared(
        &mut self,
        found: &Resolved,
        arguments: &[Argument],
        span: Span,
    ) -> Type {
        let scope = self.checker.scope_of(found);

        let (parameters, result) = if found.kind == SymbolKind::Constraint {
            let constraint = scope
                .context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name);

            (constraint.map(|c| c.parameters.as_slice()), Type::Bool)
        } else {
            let function = scope
                .context
                .functions
                .iter()
                .find(|f| f.name.name == found.name);

            let result = function.and_then(|f| f.return_type.as_ref());
            let result = result.map_or(Type::Unit, |ty| self.checker.lower_type(&scope, ty));

            (function.and_then(|f| f.parameters.as_deref()), result)
        };

        let declared = parameters.unwrap_or(&[]);

        let parameters: Vec<(String, Type)> = (declared.iter())
            .map(|p| (p.name.name.clone(), self.checker.parameter_type(&scope, p)))
            .collect();

        self.check_arguments(arguments, &parameters, &found.name, span, false);
        let constraints = Self::parameter_constraints(declared);
        self.check_known_arguments(arguments, &constraints, &scope, &found.name);

        if matches!(found.kind, SymbolKind::Function | SymbolKind::Translation) {
            self.check_known_result(found, arguments, span);
            self.check_known_call_faults(found, arguments, span);
        }

        let function = (scope.context.functions.iter()).find(|f| f.name.name == found.name);

        if let (Some(function), false) = (function, found.kind == SymbolKind::Constraint) {
            self.check_known_contract(function, &scope, arguments, &found.name, span);
        }

        result
    }
}
