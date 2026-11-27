//! Calls on the ports and repositories a service uses.

use croft_syntax::ast::{Argument, ExprKind, FunctionDecl, Identifier, Parameter};

use super::BodyChecker;
use crate::check::known::Known;
use crate::facts::MemberKind;
use crate::types::{Resolved, Type};

/// The argument given for the parameter at `index`: by its name, or in its
/// place among the arguments without one, as `check_arguments` matches them.
fn argument_for<'e>(
    arguments: &'e [Argument],
    parameters: &[Parameter],
    index: usize,
) -> Option<&'e Argument> {
    let name = &parameters.get(index)?.name.name;

    if let Some(named) =
        (arguments.iter()).find(|a| a.name.as_ref().is_some_and(|n| n.name == *name))
    {
        return Some(named);
    }

    arguments.iter().filter(|a| a.name.is_none()).nth(index)
}

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A call to a port operation, checked against its signature, and its
    /// known arguments against its parameters' constraints and contract.
    pub(super) fn port_operation(
        &mut self,
        port: &Resolved,
        method: &Identifier,
        arguments: &[Argument],
    ) -> Type {
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

        let declared = operation.parameters.as_deref().unwrap_or(&[]);
        let constraints = Self::parameter_constraints(declared);
        let callee = format!("{}.{}", port.name, method.name);
        self.check_known_arguments(arguments, &constraints, &scope, &callee);
        self.check_known_contract(operation, &scope, arguments, &callee, method.span);
        self.note_result_length(port, operation, arguments, &callee);

        let result = (operation.return_type.as_ref())
            .map_or(Type::Unit, |ty| self.checker.lower_type(&scope, ty));
        let receiver = Type::Port(port.clone());
        let defined = Some(operation.name.span);
        self.note_member(method, &receiver, &result, MemberKind::Operation, defined);

        result
    }

    /// Notes how much room a call's result needs when the operation's result
    /// has a `Length` that names a parameter, as `Files.read(path, most)`
    /// does. That argument must be known when the program is compiled.
    fn note_result_length(
        &mut self,
        port: &Resolved,
        operation: &FunctionDecl,
        arguments: &[Argument],
        callee: &str,
    ) {
        let length =
            (operation.return_constraints.iter()).find(|c| !c.each && c.name.text() == "Length");

        let Some(ExprKind::Name(named)) = length.and_then(|l| l.arguments.last()).map(|a| &a.kind)
        else {
            return;
        };

        let parameters = operation.parameters.as_deref().unwrap_or(&[]);

        let Some(index) = parameters.iter().position(|p| p.name.name == *named) else {
            return;
        };

        let Some(argument) = argument_for(arguments, parameters, index) else {
            return;
        };

        let asked = match self.known_local_value(&argument.value) {
            Some(Known::Number { units, places: 0 }) => i64::try_from(units).ok(),
            _ => None,
        };

        let Some(asked) = asked else {
            let message = format!(
                "`{named}` must be known when the program is compiled, since it says how \
                 much room `{callee}` keeps for its result: use a number or a constant"
            );

            self.error(message, argument.value.span);
            return;
        };

        let key = (port.clone(), operation.name.name.clone());
        let lengths = &mut self.checker.facts.result_lengths;
        let most = lengths.entry(key).or_insert(asked);
        *most = (*most).max(asked);
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
