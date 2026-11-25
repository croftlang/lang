//! Calls on the ports and repositories a service uses.

use croft_syntax::ast::{Argument, Identifier};

use super::BodyChecker;
use crate::facts::MemberKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn port_operation(
        &mut self,
        port: &Resolved,
        method: &Identifier,
        arguments: &[Argument],
    ) -> Type {
        if port.context.is_empty() {
            let operation = match (port.name.as_str(), method.name.as_str()) {
                ("Arguments", "all") => Some((vec![], Type::list(Type::String))),
                ("Console", "print") => Some((vec![("text", Type::String)], Type::Unit)),
                _ => None,
            };
            let Some((parameters, result)) = operation else {
                let message = format!("port `{}` has no operation `{}`", port.name, method.name);
                self.error(message, method.span);
                return Type::Unknown;
            };
            let parameters: Vec<(String, Type)> = (parameters.into_iter())
                .map(|(name, ty)| (name.to_string(), ty))
                .collect();
            self.check_arguments(arguments, &parameters, &method.name, method.span, false);
            let receiver = Type::Port(port.clone());
            self.note_member(method, &receiver, &result, MemberKind::Operation, None);
            return result;
        }
        let scope = self.checker.scope_of(port);
        let decl = scope
            .context
            .ports
            .iter()
            .find(|p| p.name.name == port.name);
        let operation =
            decl.and_then(|p| p.operations.iter().find(|op| op.name.name == method.name));
        let Some(operation) = operation else {
            self.error(
                format!("port `{}` has no operation `{}`", port.name, method.name),
                method.span,
            );
            return Type::Unknown;
        };
        let parameters: Vec<(String, Type)> = (operation.parameters.iter().flatten())
            .map(|p| (p.name.name.clone(), self.checker.parameter_type(&scope, p)))
            .collect();
        self.check_arguments(arguments, &parameters, &method.name, method.span, false);
        let result = (operation.return_type.as_ref())
            .map_or(Type::Unit, |ty| self.checker.lower_type(&scope, ty));
        let receiver = Type::Port(port.clone());
        let defined = Some(operation.name.span);
        self.note_member(method, &receiver, &result, MemberKind::Operation, defined);
        result
    }

    /// A repository gives `find(id)` and `save(aggregate)`.
    pub(super) fn repository_operation(
        &mut self,
        repository: &Resolved,
        method: &Identifier,
        arguments: &[Argument],
    ) -> Type {
        let Some(aggregate) = self.checker.repository_aggregate(repository) else {
            return Type::Unknown;
        };
        let errors = self
            .checker
            .lookup(&aggregate.context, &format!("{}.Error", aggregate.name));
        let errors = errors.map_or(Type::Unknown, Type::Named);
        let span = method.span;
        let found = match method.name.as_str() {
            "find" => {
                let identity = self.checker.identity_of(&aggregate);
                let errors = Type::Errors(vec![errors, self.checker.storage_error()]);
                let found = Type::result(Type::Named(aggregate.clone()), errors);
                self.fixed("find", arguments, &[identity], found, span)
            }
            "save" => {
                let storage = self.checker.storage_error();
                let found = Type::result(Type::Unit, storage);
                self.fixed(
                    "save",
                    arguments,
                    &[Type::Named(aggregate.clone())],
                    found,
                    span,
                )
            }
            _ => None,
        };
        let Some(found) = found else {
            let message = format!(
                "repository `{}` only has `find` and `save`",
                repository.name
            );
            self.error(message, span);
            return Type::Unknown;
        };
        let receiver = Type::Repository(repository.clone());
        self.note_member(method, &receiver, &found, MemberKind::Operation, None);
        found
    }
}
