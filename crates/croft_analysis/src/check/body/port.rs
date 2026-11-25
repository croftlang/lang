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

    /// A repository gives the operations it declares, or `find(id)` and
    /// `save(aggregate)` when it declares none.
    pub(super) fn repository_operation(
        &mut self,
        repository: &Resolved,
        method: &Identifier,
        arguments: &[Argument],
    ) -> Type {
        let operations = self.checker.operations_of(repository);
        let Some(operation) = operations.iter().find(|op| op.name == method.name) else {
            let names: Vec<String> = operations
                .iter()
                .map(|op| format!("`{}`", op.name))
                .collect();
            let message = format!(
                "repository `{}` has no operation `{}`; it has {}",
                repository.name,
                method.name,
                names.join(", ")
            );
            self.error(message, method.span);
            return Type::Unknown;
        };
        let parameters: Vec<(String, Type)> = (operation.parameters.iter())
            .map(|(name, ty, _)| (name.clone(), ty.clone()))
            .collect();
        self.check_arguments(arguments, &parameters, &method.name, method.span, false);
        let scope = self.checker.scope_of(repository);
        let declared = (scope.context.repositories.iter())
            .find(|r| r.name.name == repository.name)
            .and_then(|r| r.operations.as_ref())
            .and_then(|ops| ops.iter().find(|op| op.name.name == method.name))
            .map(|op| op.name.span);
        let result = operation.result.clone();
        let receiver = Type::Repository(repository.clone());
        self.note_member(method, &receiver, &result, MemberKind::Operation, declared);
        result
    }
}
